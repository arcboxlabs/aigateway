//! Request translation: canonical `ChatRequest` → OpenAI HTTP request.
//!
//! The canonical format is OpenAI-native, so translation is direct serialization.
//! The translator's main job is building the correct URL, auth headers, and body.

use std::collections::BTreeMap;

use aigw_core::error::TranslateError;
use aigw_core::model::ChatRequest;
use aigw_core::translate::{RequestTranslator, TranslatedRequest};
use bytes::Bytes;
use http::{HeaderMap, HeaderName, HeaderValue, Method};

use crate::transport::OpenAITransport;

/// Translates canonical requests into OpenAI Chat Completions API requests.
///
/// Holds an [`OpenAITransport`] for URL and header construction. The transport
/// is cloned at construction time, so the translator is self-contained.
pub struct OpenAIRequestTranslator {
    transport: OpenAITransport,
}

impl OpenAIRequestTranslator {
    pub fn new(transport: OpenAITransport) -> Self {
        Self { transport }
    }
}

impl RequestTranslator for OpenAIRequestTranslator {
    fn translate_request(&self, req: &ChatRequest) -> Result<TranslatedRequest, TranslateError> {
        let body = serde_json::to_vec(req)?;
        let transport_req = self
            .transport
            .prepare_json_request("/chat/completions", &BTreeMap::new());
        let headers = btree_to_headermap(&transport_req.headers)?;

        Ok(TranslatedRequest {
            url: transport_req.url,
            method: Method::POST,
            headers,
            body: Bytes::from(body),
        })
    }

    fn translate_stream_request(
        &self,
        req: &ChatRequest,
    ) -> Result<TranslatedRequest, TranslateError> {
        // Serialize to a mutable JSON value so we can inject stream fields.
        let mut json = serde_json::to_value(req)?;
        if let Some(obj) = json.as_object_mut() {
            obj.insert("stream".into(), serde_json::Value::Bool(true));
            obj.insert(
                "stream_options".into(),
                serde_json::json!({ "include_usage": true }),
            );
        }
        let body = serde_json::to_vec(&json)?;

        let mut extra_headers = BTreeMap::new();
        extra_headers.insert("Accept".to_owned(), "text/event-stream".to_owned());
        let transport_req = self
            .transport
            .prepare_json_request("/chat/completions", &extra_headers);
        let headers = btree_to_headermap(&transport_req.headers)?;

        Ok(TranslatedRequest {
            url: transport_req.url,
            method: Method::POST,
            headers,
            body: Bytes::from(body),
        })
    }
}

/// Convert `BTreeMap<String, String>` (from transport) into `http::HeaderMap`.
fn btree_to_headermap(map: &BTreeMap<String, String>) -> Result<HeaderMap, TranslateError> {
    let mut headers = HeaderMap::with_capacity(map.len());
    for (name, value) in map {
        let name = HeaderName::try_from(name.as_str())
            .map_err(|e| TranslateError::Other(format!("invalid header name '{name}': {e}")))?;
        let value = HeaderValue::try_from(value.as_str())
            .map_err(|e| TranslateError::Other(format!("invalid header value: {e}")))?;
        headers.insert(name, value);
    }
    Ok(headers)
}

#[cfg(test)]
mod tests {
    use aigw_core::model::{
        ChatRequest, Message, MessageContent, Role, Tool, ToolChoice, ToolChoiceMode,
    };

    #[test]
    fn serialize_minimal_request() {
        let req = ChatRequest {
            model: "gpt-4.1".into(),
            messages: vec![Message {
                role: Role::User,
                content: Some(MessageContent::Text("Hello".into())),
                name: None,
                tool_call_id: None,
                tool_calls: None,
                extra: Default::default(),
            }],
            temperature: None,
            max_tokens: None,
            top_p: None,
            stop: None,
            stream: None,
            tools: None,
            tool_choice: None,
            response_format: None,
            frequency_penalty: None,
            presence_penalty: None,
            n: None,
            seed: None,
            user: None,
            extra: Default::default(),
        };

        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["model"], "gpt-4.1");
        assert_eq!(json["messages"][0]["role"], "user");
        assert_eq!(json["messages"][0]["content"], "Hello");
        // Optional fields should be absent
        assert!(json.get("temperature").is_none());
        assert!(json.get("tools").is_none());
    }

    #[test]
    fn extra_fields_pass_through_to_wire() {
        let mut extra = serde_json::Map::new();
        extra.insert("logprobs".into(), serde_json::Value::Bool(true));
        extra.insert(
            "max_completion_tokens".into(),
            serde_json::Value::Number(8192.into()),
        );

        let req = ChatRequest {
            model: "gpt-4.1".into(),
            messages: vec![],
            extra,
            temperature: None,
            max_tokens: None,
            top_p: None,
            stop: None,
            stream: None,
            tools: None,
            tool_choice: None,
            response_format: None,
            frequency_penalty: None,
            presence_penalty: None,
            n: None,
            seed: None,
            user: None,
        };

        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["logprobs"], true);
        assert_eq!(json["max_completion_tokens"], 8192);
    }

    #[test]
    fn tool_choice_serializes_correctly() {
        let req = ChatRequest {
            model: "gpt-4.1".into(),
            messages: vec![],
            tool_choice: Some(ToolChoice::Mode(ToolChoiceMode::Required)),
            tools: Some(vec![Tool {
                kind: "function".into(),
                function: aigw_core::model::FunctionDefinition {
                    name: "get_weather".into(),
                    description: Some("Get weather".into()),
                    parameters: Some(serde_json::json!({"type": "object"})),
                    strict: None,
                    extra: Default::default(),
                },
                extra: Default::default(),
            }]),
            temperature: None,
            max_tokens: None,
            top_p: None,
            stop: None,
            stream: None,
            response_format: None,
            frequency_penalty: None,
            presence_penalty: None,
            n: None,
            seed: None,
            user: None,
            extra: Default::default(),
        };

        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["tool_choice"], "required");
        assert_eq!(json["tools"][0]["function"]["name"], "get_weather");
    }
}
