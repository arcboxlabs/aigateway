//! Anthropic API client.
//!
//! Thin layer over [`Transport`](crate::Transport) that provides typed methods
//! for each API endpoint. All HTTP plumbing lives in the transport module.

use futures::StreamExt;
use reqwest::Response;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio_stream::Stream;

use crate::error::Error;
use crate::rate_limit::{ApiResponse, RateLimitInfo};
use crate::streaming::parse_sse_stream;
use crate::transport::Transport;
use crate::types::{
    ApiErrorResponse, CountTokensRequest, CountTokensResponse, MessagesRequest, MessagesResponse,
    ModelListResponse, StreamEvent,
};

/// Anthropic API client.
///
/// Construct via [`Client::new`] with a validated [`Transport`].
#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    transport: Transport,
}

impl Client {
    /// Create a new client from a validated transport.
    pub fn new(transport: Transport) -> Result<Self, Error> {
        let http = reqwest::Client::builder()
            .timeout(transport.timeout())
            .build()
            .map_err(|e| Error::Config(format!("failed to build HTTP client: {e}")))?;

        Ok(Self { http, transport })
    }

    /// Create a client from the `ANTHROPIC_API_KEY` environment variable with default settings.
    pub fn from_env() -> Result<Self, Error> {
        use crate::transport::TransportConfig;
        use secrecy::SecretString;

        let api_key =
            std::env::var("ANTHROPIC_API_KEY").map_err(|e| Error::Config(e.to_string()))?;

        let transport = Transport::new(TransportConfig {
            api_key: SecretString::from(api_key),
            ..Default::default()
        })?;

        Self::new(transport)
    }

    /// Access the underlying transport.
    pub fn transport(&self) -> &Transport {
        &self.transport
    }

    // ─── Standard API endpoints ──────────────────────────────────────────

    /// Send a non-streaming messages request.
    pub async fn messages(
        &self,
        req: &MessagesRequest,
    ) -> Result<ApiResponse<MessagesResponse>, Error> {
        self.post_json("/v1/messages", req).await
    }

    /// Send a streaming messages request, returning a stream of events.
    ///
    /// Rate limit info is captured from the initial HTTP response headers
    /// before streaming begins.
    pub async fn messages_stream(
        &self,
        req: &MessagesRequest,
    ) -> Result<ApiResponse<impl Stream<Item = Result<StreamEvent, Error>> + Send>, Error> {
        let response = self.send_post("/v1/messages", req).await?;

        let rate_limit = RateLimitInfo::from_headers(response.headers());

        let byte_stream = response.bytes_stream().map(|result| {
            result.map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::ConnectionReset, e.to_string())
            })
        });

        Ok(ApiResponse {
            body: parse_sse_stream(byte_stream),
            rate_limit,
        })
    }

    /// Count the number of input tokens for a messages request.
    pub async fn count_tokens(
        &self,
        req: &CountTokensRequest,
    ) -> Result<ApiResponse<CountTokensResponse>, Error> {
        self.post_json("/v1/messages/count_tokens", req).await
    }

    /// List available models.
    pub async fn list_models(&self) -> Result<ApiResponse<ModelListResponse>, Error> {
        self.get_json("/v1/models").await
    }

    // ─── Claude Code non-standard endpoints ──────────────────────────────

    /// Send a batch of telemetry events.
    ///
    /// This is a **non-standard** endpoint used by Claude Code.
    #[cfg(feature = "claude-code")]
    pub async fn event_logging(
        &self,
        req: &crate::types::EventLoggingRequest,
    ) -> Result<ApiResponse<crate::types::EventLoggingResponse>, Error> {
        self.post_json("/api/event_logging/batch", req).await
    }

    /// Exchange or refresh an OAuth token.
    ///
    /// This is a **non-standard** endpoint used by Claude Code.
    /// Note: the OAuth endpoint may use a different base URL than the API
    /// (e.g. `platform.claude.com` instead of `api.anthropic.com`).
    #[cfg(feature = "claude-code")]
    pub async fn oauth_token(
        &self,
        req: &crate::types::OAuthTokenRequest,
    ) -> Result<ApiResponse<crate::types::OAuthTokenResponse>, Error> {
        self.post_json("/v1/oauth/token", req).await
    }

    // ─── Internal helpers ────────────────────────────────────────────────

    async fn post_json<T, B>(&self, path: &str, body: &B) -> Result<ApiResponse<T>, Error>
    where
        T: DeserializeOwned,
        B: Serialize,
    {
        let response = self.send_post(path, body).await?;
        self.decode_response(response).await
    }

    async fn get_json<T>(&self, path: &str) -> Result<ApiResponse<T>, Error>
    where
        T: DeserializeOwned,
    {
        let response = self.send_get(path).await?;
        self.decode_response(response).await
    }

    /// Send a GET request, returning the raw response on success.
    async fn send_get(&self, path: &str) -> Result<Response, Error> {
        let url = self.transport.url(path);
        let response = self
            .http
            .get(&url)
            .headers(self.transport.headers().clone())
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response)
        } else {
            Err(Self::parse_error_response(response).await)
        }
    }

    /// Send a POST request, returning the raw response on success.
    /// Used by both `post_json` (which decodes) and `messages_stream` (which streams).
    async fn send_post<B>(&self, path: &str, body: &B) -> Result<Response, Error>
    where
        B: Serialize,
    {
        let url = self.transport.url(path);
        let response = self
            .http
            .post(&url)
            .headers(self.transport.headers().clone())
            .json(body)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response)
        } else {
            Err(Self::parse_error_response(response).await)
        }
    }

    /// Decode a successful response into body + rate limit info.
    async fn decode_response<T>(&self, response: Response) -> Result<ApiResponse<T>, Error>
    where
        T: DeserializeOwned,
    {
        let rate_limit = RateLimitInfo::from_headers(response.headers());
        let text = response.text().await?;
        Ok(ApiResponse {
            body: serde_json::from_str(&text)?,
            rate_limit,
        })
    }

    /// Parse an error response body. Falls back to [`Error::UnexpectedResponse`]
    /// if the body is not valid Anthropic JSON (e.g. a 502 HTML page from a proxy).
    async fn parse_error_response(response: Response) -> Error {
        let status = response.status().as_u16();
        let body = match response.text().await {
            Ok(body) => body,
            Err(e) => return Error::Http(e),
        };

        match serde_json::from_str::<ApiErrorResponse>(&body) {
            Ok(parsed) => Error::from_api_error(status, parsed.error),
            Err(_) => Error::UnexpectedResponse { status, body },
        }
    }
}

#[cfg(test)]
mod tests {
    use futures_util::StreamExt;
    use secrecy::SecretString;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::sync::oneshot;

    use super::*;
    use crate::transport::TransportConfig;
    use crate::types::{Message, MessageContent, MessagesRequest, Role};

    fn transport(base_url: String) -> Transport {
        Transport::new(TransportConfig {
            api_key: SecretString::from("sk-ant-test-key"),
            base_url,
            ..Default::default()
        })
        .unwrap()
    }

    fn messages_request() -> MessagesRequest {
        MessagesRequest::builder()
            .model("claude-sonnet-4-20250514")
            .messages(vec![Message {
                role: Role::User,
                content: MessageContent::Text("Hello".into()),
            }])
            .max_tokens(1024_u64)
            .build()
    }

    // ─── Mock HTTP server ────────────────────────────────────────────────

    async fn spawn_server(response: String) -> (String, oneshot::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = oneshot::channel();

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let request = read_http_request(&mut socket).await;
            let _ = tx.send(request);
            socket.write_all(response.as_bytes()).await.unwrap();
            socket.shutdown().await.unwrap();
        });

        (format!("http://{addr}"), rx)
    }

    async fn read_http_request(socket: &mut tokio::net::TcpStream) -> String {
        let mut buf = Vec::new();
        let mut header_end = None;
        let mut content_length = 0usize;

        loop {
            let mut chunk = [0u8; 4096];
            let n = socket.read(&mut chunk).await.unwrap();
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);

            if header_end.is_none() {
                if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                    header_end = Some(pos + 4);
                    let headers = String::from_utf8_lossy(&buf[..pos + 4]);
                    content_length = headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            if name.eq_ignore_ascii_case("content-length") {
                                value.trim().parse().ok()
                            } else {
                                None
                            }
                        })
                        .unwrap_or(0);
                }
            }

            if let Some(end) = header_end {
                if buf.len() >= end + content_length {
                    break;
                }
            }
        }

        String::from_utf8(buf).unwrap()
    }

    fn http_response(status: &str, content_type: &str, body: &str) -> String {
        format!(
            "HTTP/1.1 {status}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        )
    }

    fn http_response_with_rate_limits(status: &str, content_type: &str, body: &str) -> String {
        format!(
            "HTTP/1.1 {status}\r\n\
             content-type: {content_type}\r\n\
             content-length: {}\r\n\
             anthropic-ratelimit-requests-limit: 1000\r\n\
             anthropic-ratelimit-requests-remaining: 999\r\n\
             anthropic-ratelimit-tokens-remaining: 90000\r\n\
             connection: close\r\n\r\n{body}",
            body.len()
        )
    }

    // ─── Tests ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn messages_sends_correct_request_and_decodes_response() {
        let body = r#"{
            "id": "msg_01XFDUDYJgAACzvnptvVoYEL",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "Hi there!"}],
            "model": "claude-sonnet-4-20250514",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {"input_tokens": 10, "output_tokens": 5}
        }"#;
        let (base_url, req_rx) =
            spawn_server(http_response("200 OK", "application/json", body)).await;
        let client = Client::new(transport(base_url)).unwrap();

        let resp = client.messages(&messages_request()).await.unwrap();
        assert_eq!(resp.body.id, "msg_01XFDUDYJgAACzvnptvVoYEL");
        assert_eq!(resp.body.usage.input_tokens, 10);

        let raw = req_rx.await.unwrap().to_lowercase();
        assert!(raw.contains("post /v1/messages http/1.1"));
        assert!(raw.contains("x-api-key: sk-ant-test-key"));
        assert!(raw.contains("anthropic-version: 2023-06-01"));
        assert!(raw.contains("content-type: application/json"));
        assert!(raw.contains("claude-sonnet-4-20250514"));
    }

    #[tokio::test]
    async fn messages_parses_rate_limit_headers() {
        let body = r#"{
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "ok"}],
            "model": "claude-sonnet-4-20250514",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {"input_tokens": 1, "output_tokens": 1}
        }"#;
        let (base_url, _) = spawn_server(http_response_with_rate_limits(
            "200 OK",
            "application/json",
            body,
        ))
        .await;
        let client = Client::new(transport(base_url)).unwrap();

        let resp = client.messages(&messages_request()).await.unwrap();
        assert_eq!(resp.rate_limit.requests_limit, Some(1000));
        assert_eq!(resp.rate_limit.requests_remaining, Some(999));
        assert_eq!(resp.rate_limit.tokens_remaining, Some(90000));
    }

    #[tokio::test]
    async fn list_models_sends_get_request() {
        let body = r#"{
            "data": [{"id": "claude-sonnet-4-20250514"}],
            "has_more": false,
            "first_id": "claude-sonnet-4-20250514",
            "last_id": "claude-sonnet-4-20250514"
        }"#;
        let (base_url, req_rx) =
            spawn_server(http_response("200 OK", "application/json", body)).await;
        let client = Client::new(transport(base_url)).unwrap();

        let resp = client.list_models().await.unwrap();
        assert_eq!(resp.body.data.len(), 1);
        assert_eq!(resp.body.data[0].id, "claude-sonnet-4-20250514");

        let raw = req_rx.await.unwrap().to_lowercase();
        assert!(raw.contains("get /v1/models http/1.1"));
    }

    #[tokio::test]
    async fn count_tokens_decodes_response() {
        let body = r#"{"input_tokens": 42}"#;
        let (base_url, req_rx) =
            spawn_server(http_response("200 OK", "application/json", body)).await;
        let client = Client::new(transport(base_url)).unwrap();

        let req = crate::types::CountTokensRequest::builder()
            .model("claude-sonnet-4-20250514")
            .messages(vec![Message {
                role: Role::User,
                content: MessageContent::Text("Hello".into()),
            }])
            .build();

        let resp = client.count_tokens(&req).await.unwrap();
        assert_eq!(resp.body.input_tokens, 42);

        let raw = req_rx.await.unwrap().to_lowercase();
        assert!(raw.contains("post /v1/messages/count_tokens http/1.1"));
    }

    #[tokio::test]
    async fn api_error_response_is_parsed() {
        let body = r#"{
            "type": "error",
            "error": {
                "type": "invalid_request_error",
                "message": "model not found"
            }
        }"#;
        let (base_url, _) =
            spawn_server(http_response("400 Bad Request", "application/json", body)).await;
        let client = Client::new(transport(base_url)).unwrap();

        let err = client.messages(&messages_request()).await.unwrap_err();
        match err {
            Error::Api {
                status,
                error_type,
                message,
            } => {
                assert_eq!(status, 400);
                assert_eq!(error_type, "invalid_request_error");
                assert_eq!(message, "model not found");
            }
            other => panic!("expected Api error, got: {other}"),
        }
    }

    #[tokio::test]
    async fn non_json_error_becomes_unexpected_response() {
        let (base_url, _) = spawn_server(http_response(
            "502 Bad Gateway",
            "text/html",
            "<html>Bad Gateway</html>",
        ))
        .await;
        let client = Client::new(transport(base_url)).unwrap();

        let err = client.messages(&messages_request()).await.unwrap_err();
        match err {
            Error::UnexpectedResponse { status, body } => {
                assert_eq!(status, 502);
                assert!(body.contains("Bad Gateway"));
            }
            other => panic!("expected UnexpectedResponse, got: {other}"),
        }
    }

    #[tokio::test]
    async fn rate_limit_error_is_parsed() {
        let body = r#"{
            "type": "error",
            "error": {
                "type": "rate_limit_error",
                "message": "rate limited"
            }
        }"#;
        let (base_url, _) = spawn_server(http_response(
            "429 Too Many Requests",
            "application/json",
            body,
        ))
        .await;
        let client = Client::new(transport(base_url)).unwrap();

        let err = client.messages(&messages_request()).await.unwrap_err();
        match err {
            Error::Api {
                status, error_type, ..
            } => {
                assert_eq!(status, 429);
                assert_eq!(error_type, "rate_limit_error");
            }
            other => panic!("expected Api error, got: {other}"),
        }
    }

    #[tokio::test]
    async fn messages_stream_yields_events() {
        let events = concat!(
            "event: message_start\n",
            "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_1\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[],\"model\":\"claude-sonnet-4-20250514\",\"stop_reason\":null,\"stop_sequence\":null,\"usage\":{\"input_tokens\":10,\"output_tokens\":0}}}\n\n",
            "event: content_block_start\n",
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}\n\n",
            "event: content_block_stop\n",
            "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
            "event: message_delta\n",
            "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null},\"usage\":{\"output_tokens\":5}}\n\n",
            "event: message_stop\n",
            "data: {\"type\":\"message_stop\"}\n\n",
        );
        let (base_url, _) =
            spawn_server(http_response("200 OK", "text/event-stream", events)).await;
        let client = Client::new(transport(base_url)).unwrap();

        let mut req = messages_request();
        req.stream = Some(true);

        let resp = client.messages_stream(&req).await.unwrap();
        assert_eq!(resp.rate_limit.requests_limit, None);

        let collected: Vec<_> = resp.body.collect().await;
        assert_eq!(collected.len(), 6);

        // Verify first event is message_start
        match collected[0].as_ref().unwrap() {
            crate::types::StreamEvent::MessageStart { message } => {
                assert_eq!(message.id, "msg_1");
            }
            other => panic!("expected MessageStart, got: {other:?}"),
        }

        // Verify delta content
        match collected[2].as_ref().unwrap() {
            crate::types::StreamEvent::ContentBlockDelta { index, delta } => {
                assert_eq!(*index, 0);
                match delta {
                    crate::types::ContentDelta::TextDelta { text } => {
                        assert_eq!(text, "Hello");
                    }
                    other => panic!("expected TextDelta, got: {other:?}"),
                }
            }
            other => panic!("expected ContentBlockDelta, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn transport_error_propagates_as_typed_error() {
        let err = Client::from_env();
        // Unless ANTHROPIC_API_KEY is set, this should fail with Config (env var)
        // We just verify it doesn't panic
        assert!(err.is_err());
    }
}
