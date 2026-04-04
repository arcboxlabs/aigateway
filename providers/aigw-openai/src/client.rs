use std::pin::Pin;

use futures_core::Stream;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Method, Response};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::{OpenAIApiError, OpenAIApiErrorKind, OpenAIError};
use crate::sse::parse_openai_sse;
use crate::transport::{OpenAITransport, OpenAITransportConfig, OpenAITransportConfigError};
use crate::wire_types::{
    ApiErrorResponse, ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, Model,
    ModelListResponse,
};

pub type OpenAIChatCompletionStream =
    Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk, OpenAIError>> + Send>>;

#[derive(Clone, Debug)]
pub struct OpenAIClient {
    http: reqwest::Client,
    transport: OpenAITransport,
}

impl OpenAIClient {
    pub fn new(config: OpenAITransportConfig) -> Result<Self, OpenAITransportConfigError> {
        Ok(Self::from_transport(OpenAITransport::new(config)?))
    }

    pub fn from_transport(transport: OpenAITransport) -> Self {
        Self {
            http: reqwest::Client::new(),
            transport,
        }
    }

    pub fn with_http_client(http: reqwest::Client, transport: OpenAITransport) -> Self {
        Self { http, transport }
    }

    pub fn transport(&self) -> &OpenAITransport {
        &self.transport
    }

    pub async fn create_chat_completion(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, OpenAIError> {
        self.post_json("/chat/completions", request).await
    }

    pub async fn stream_chat_completion(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<OpenAIChatCompletionStream, OpenAIError> {
        let mut streamed_request = request.clone();
        streamed_request.stream = Some(true);

        let response = self
            .post_json_response("/chat/completions", &streamed_request)
            .await?;
        let stream = parse_openai_sse::<_, _, _, ChatCompletionChunk>(response.bytes_stream())
            .map(|item| item.map_err(OpenAIError::from));

        Ok(Box::pin(stream))
    }

    pub async fn list_models(&self) -> Result<ModelListResponse, OpenAIError> {
        self.get_json("/models").await
    }

    pub async fn get_model(&self, model: &str) -> Result<Model, OpenAIError> {
        self.get_json(&format!("/models/{model}")).await
    }

    async fn get_json<T>(&self, path: &str) -> Result<T, OpenAIError>
    where
        T: DeserializeOwned,
    {
        let prepared = self
            .transport
            .prepare_request(path, &HeaderMapSpec::json_accept_only());
        let request = self.request_builder(Method::GET, prepared)?;
        let response = self.execute(request).await?;
        self.decode_json(response).await
    }

    async fn post_json<T, B>(&self, path: &str, body: &B) -> Result<T, OpenAIError>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        let response = self.post_json_response(path, body).await?;
        self.decode_json(response).await
    }

    async fn post_json_response<B>(&self, path: &str, body: &B) -> Result<Response, OpenAIError>
    where
        B: Serialize + ?Sized,
    {
        let prepared = self
            .transport
            .prepare_json_request(path, &HeaderMapSpec::empty());
        let request = self.request_builder(Method::POST, prepared)?.json(body);
        self.execute(request).await
    }

    fn request_builder(
        &self,
        method: Method,
        prepared: crate::transport::OpenAITransportRequest,
    ) -> Result<reqwest::RequestBuilder, OpenAIError> {
        Ok(self
            .http
            .request(method, prepared.url)
            .headers(build_header_map(&prepared.headers)?))
    }

    async fn execute(&self, request: reqwest::RequestBuilder) -> Result<Response, OpenAIError> {
        let response = request.send().await.map_err(OpenAIError::from_reqwest)?;

        if response.status().is_success() {
            Ok(response)
        } else {
            Err(api_error_from_response(response).await)
        }
    }

    async fn decode_json<T>(&self, response: Response) -> Result<T, OpenAIError>
    where
        T: DeserializeOwned,
    {
        let bytes = response.bytes().await.map_err(OpenAIError::from_reqwest)?;
        serde_json::from_slice(&bytes).map_err(|source| OpenAIError::Decode {
            source,
            body: String::from_utf8_lossy(&bytes).into_owned(),
        })
    }
}

fn build_header_map(
    headers: &std::collections::BTreeMap<String, String>,
) -> Result<HeaderMap, OpenAIError> {
    let mut header_map = HeaderMap::new();

    for (name, value) in headers {
        let header_name =
            HeaderName::from_bytes(name.as_bytes()).map_err(|_| OpenAIError::InvalidHeader {
                name: name.clone(),
                value: value.clone(),
            })?;
        let header_value =
            HeaderValue::from_str(value).map_err(|_| OpenAIError::InvalidHeader {
                name: name.clone(),
                value: value.clone(),
            })?;
        header_map.insert(header_name, header_value);
    }

    Ok(header_map)
}

async fn api_error_from_response(response: Response) -> OpenAIError {
    let status = response.status().as_u16();
    let request_id = response
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let bytes = match response.bytes().await {
        Ok(bytes) => bytes,
        Err(error) => return OpenAIError::from_reqwest(error),
    };
    let body = String::from_utf8_lossy(&bytes).into_owned();

    let (message, error_type, param, code) =
        match serde_json::from_slice::<ApiErrorResponse>(&bytes) {
            Ok(parsed) => (
                parsed.error.message,
                parsed.error.kind,
                parsed.error.param,
                parsed.error.code,
            ),
            Err(_) => (body.clone(), None, None, None),
        };

    OpenAIError::Api(OpenAIApiError {
        kind: OpenAIApiErrorKind::from_status(status),
        status,
        message,
        error_type,
        param,
        code,
        request_id,
        body,
    })
}

struct HeaderMapSpec;

impl HeaderMapSpec {
    fn empty() -> std::collections::BTreeMap<String, String> {
        std::collections::BTreeMap::new()
    }

    fn json_accept_only() -> std::collections::BTreeMap<String, String> {
        std::collections::BTreeMap::from([("Accept".to_owned(), "application/json".to_owned())])
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use futures_util::StreamExt;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};
    use tokio::sync::oneshot;

    use super::OpenAIClient;
    use crate::error::{OpenAIApiErrorKind, OpenAIError};
    use crate::transport::{HttpTransportConfig, OpenAIAuthConfig, OpenAITransportConfig};
    use crate::wire_types::{
        ChatCompletionRequest, ChatMessage, ChatMessageContent, ChatMessageRole,
    };

    fn config(base_url: String) -> OpenAITransportConfig {
        OpenAITransportConfig {
            http: HttpTransportConfig {
                base_url,
                default_headers: BTreeMap::from([("X-Default".to_owned(), "default".to_owned())]),
            },
            auth: OpenAIAuthConfig {
                api_key: "sk-test".to_owned(),
                organization: Some("org_123".to_owned()),
                project: Some("proj_456".to_owned()),
            },
        }
    }

    #[tokio::test]
    async fn list_models_decodes_response_and_sends_headers() {
        let body = r#"{
            "object":"list",
            "data":[{"id":"gpt-4.1","object":"model","owned_by":"openai"}]
        }"#;
        let (base_url, request_rx) =
            spawn_server(http_response("200 OK", "application/json", body)).await;
        let client = OpenAIClient::new(config(base_url)).unwrap();

        let response = client.list_models().await.unwrap();
        assert_eq!(response.data[0].id, "gpt-4.1");

        let request = request_rx.await.unwrap().to_lowercase();
        assert!(request.contains("get /v1/models http/1.1"));
        assert!(request.contains("authorization: bearer sk-test"));
        assert!(request.contains("openai-organization: org_123"));
        assert!(request.contains("openai-project: proj_456"));
        assert!(request.contains("x-default: default"));
    }

    #[tokio::test]
    async fn stream_chat_completion_parses_sse_chunks() {
        let chunk_1 = r#"{"id":"chatcmpl_1","object":"chat.completion.chunk","created":1,"model":"gpt-4.1","choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]}"#;
        let chunk_2 = r#"{"id":"chatcmpl_1","object":"chat.completion.chunk","created":1,"model":"gpt-4.1","choices":[{"index":0,"delta":{"content":"hello"},"finish_reason":"stop"}]}"#;
        let body = format!("data: {chunk_1}\n\ndata: {chunk_2}\n\ndata: [DONE]\n\n");
        let (base_url, request_rx) =
            spawn_server(http_response("200 OK", "text/event-stream", &body)).await;
        let client = OpenAIClient::new(config(base_url)).unwrap();
        let request = ChatCompletionRequest {
            model: "gpt-4.1".to_owned(),
            messages: vec![ChatMessage {
                role: ChatMessageRole::User,
                content: Some(ChatMessageContent::Text("hi".to_owned())),
                name: None,
                refusal: None,
                tool_call_id: None,
                tool_calls: None,
                extra: BTreeMap::new(),
            }],
            frequency_penalty: None,
            logprobs: None,
            max_completion_tokens: None,
            max_tokens: None,
            metadata: None,
            n: None,
            parallel_tool_calls: None,
            presence_penalty: None,
            response_format: None,
            seed: None,
            service_tier: None,
            stop: None,
            store: None,
            stream: None,
            stream_options: None,
            temperature: None,
            tool_choice: None,
            tools: None,
            top_logprobs: None,
            top_p: None,
            user: None,
            extra: BTreeMap::new(),
        };

        let chunks = client
            .stream_chat_completion(&request)
            .await
            .unwrap()
            .collect::<Vec<_>>()
            .await;

        assert_eq!(chunks.len(), 2);
        assert_eq!(
            chunks[1].as_ref().unwrap().choices[0]
                .delta
                .content
                .as_deref(),
            Some("hello")
        );

        let request = request_rx.await.unwrap().to_lowercase();
        assert!(request.contains("post /v1/chat/completions http/1.1"));
        assert!(request.contains("\"stream\":true"));
    }

    #[tokio::test]
    async fn list_models_classifies_api_errors() {
        let body = r#"{
            "error": {
                "message": "invalid api key",
                "type": "invalid_request_error",
                "param": "api_key"
            }
        }"#;
        let (base_url, _) =
            spawn_server(http_response("401 Unauthorized", "application/json", body)).await;
        let client = OpenAIClient::new(config(base_url)).unwrap();

        let error = client.list_models().await.unwrap_err();
        match error {
            OpenAIError::Api(error) => {
                assert_eq!(error.kind, OpenAIApiErrorKind::Authentication);
                assert_eq!(error.status, 401);
                assert_eq!(error.message, "invalid api key");
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    async fn spawn_server(response: String) -> (String, oneshot::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (request_tx, request_rx) = oneshot::channel();

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let request = read_http_request(&mut socket).await;
            let _ = request_tx.send(request);
            socket.write_all(response.as_bytes()).await.unwrap();
            socket.shutdown().await.unwrap();
        });

        (format!("http://{address}/v1"), request_rx)
    }

    async fn read_http_request(socket: &mut TcpStream) -> String {
        let mut buffer = Vec::new();
        let mut header_end = None;
        let mut content_length = 0usize;

        loop {
            let mut chunk = [0u8; 1024];
            let read = socket.read(&mut chunk).await.unwrap();
            if read == 0 {
                break;
            }

            buffer.extend_from_slice(&chunk[..read]);

            if header_end.is_none() {
                if let Some(position) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
                    header_end = Some(position + 4);
                    let headers = String::from_utf8_lossy(&buffer[..position + 4]);
                    content_length = parse_content_length(&headers);
                }
            }

            if let Some(end) = header_end {
                if buffer.len() >= end + content_length {
                    break;
                }
            }
        }

        String::from_utf8(buffer).unwrap()
    }

    fn parse_content_length(headers: &str) -> usize {
        headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                if name.eq_ignore_ascii_case("content-length") {
                    value.trim().parse().ok()
                } else {
                    None
                }
            })
            .unwrap_or(0)
    }

    fn http_response(status: &str, content_type: &str, body: &str) -> String {
        format!(
            "HTTP/1.1 {status}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        )
    }
}
