use std::error::Error;
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::shared::{JsonObject, json_object_is_empty};

pub type ResponseInput = Value;
pub type ResponseTool = Value;
pub type ResponseToolChoice = Value;
pub type ResponseConversation = Value;
pub type ResponseContextManagement = Value;
pub type ResponseReasoning = Value;
pub type ResponseContentPart = Value;
pub type ResponseOutputItem = Value;
pub type ResponseInputItem = Value;
pub type ResponsePromptCacheRetention = Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseCreateRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation: Option<ResponseConversation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_management: Option<ResponseContextManagement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<ResponseInput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructions: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonObject>,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_cache_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_cache_retention: Option<ResponsePromptCacheRetention>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ResponseReasoning>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safety_identifier: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<ResponseStreamOptions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<ResponseTextConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ResponseToolChoice>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ResponseTool>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub truncation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

impl ResponseCreateRequest {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            background: None,
            conversation: None,
            context_management: None,
            include: None,
            input: None,
            instructions: None,
            max_output_tokens: None,
            metadata: None,
            model: model.into(),
            parallel_tool_calls: None,
            previous_response_id: None,
            prompt_cache_key: None,
            prompt_cache_retention: None,
            reasoning: None,
            safety_identifier: None,
            service_tier: None,
            store: None,
            stream: None,
            stream_options: None,
            temperature: None,
            text: None,
            tool_choice: None,
            tools: None,
            top_p: None,
            truncation: None,
            user: None,
            extra: JsonObject::new(),
        }
    }

    pub fn validate(&self) -> Result<(), ResponseCreateRequestError> {
        validate_previous_response_id_and_conversation(
            self.previous_response_id.as_deref(),
            self.conversation.as_ref(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseCreateRequestError {
    PreviousResponseIdAndConversationConflict,
}

impl Display for ResponseCreateRequestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::PreviousResponseIdAndConversationConflict => {
                f.write_str("`previous_response_id` and `conversation` cannot both be set")
            }
        }
    }
}

impl Error for ResponseCreateRequestError {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseInputTokensRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation: Option<ResponseConversation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<ResponseInput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructions: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ResponseReasoning>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<ResponseTextConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ResponseToolChoice>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ResponseTool>>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

impl ResponseInputTokensRequest {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            conversation: None,
            input: None,
            instructions: None,
            model: Some(model.into()),
            parallel_tool_calls: None,
            previous_response_id: None,
            reasoning: None,
            text: None,
            tool_choice: None,
            tools: None,
            extra: JsonObject::new(),
        }
    }

    pub fn validate(&self) -> Result<(), ResponseCreateRequestError> {
        validate_previous_response_id_and_conversation(
            self.previous_response_id.as_deref(),
            self.conversation.as_ref(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseInputTokensResponse {
    pub object: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<i64>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseCompactRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<ResponseInput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructions: Option<Value>,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

impl ResponseCompactRequest {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            input: None,
            instructions: None,
            model: model.into(),
            previous_response_id: None,
            extra: JsonObject::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseTextConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verbosity: Option<String>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseStreamOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_obfuscation: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_usage: Option<bool>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseObject {
    pub id: String,
    pub object: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation: Option<ResponseConversation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub incomplete_details: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructions: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonObject>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<Vec<ResponseOutputItem>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_cache_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_cache_retention: Option<ResponsePromptCacheRetention>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ResponseReasoning>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safety_identifier: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<ResponseTextConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ResponseToolChoice>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ResponseTool>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub truncation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<ResponseUsage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseUsage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_tokens_details: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_tokens_details: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<i64>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseCompaction {
    pub id: String,
    pub object: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<Vec<ResponseOutputItem>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<ResponseUsage>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseInputItemsPage {
    pub object: String,
    pub data: Vec<ResponseInputItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_more: Option<bool>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseRetrieveStreamQuery {
    pub stream: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_after: Option<String>,
}

impl Default for ResponseRetrieveStreamQuery {
    fn default() -> Self {
        Self {
            stream: true,
            starting_after: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseStreamEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

fn validate_previous_response_id_and_conversation(
    previous_response_id: Option<&str>,
    conversation: Option<&ResponseConversation>,
) -> Result<(), ResponseCreateRequestError> {
    if previous_response_id.is_some() && conversation.is_some() {
        return Err(ResponseCreateRequestError::PreviousResponseIdAndConversationConflict);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        ResponseCompactRequest, ResponseCreateRequest, ResponseInputTokensRequest,
        ResponseRetrieveStreamQuery, ResponseStreamEvent,
    };

    #[test]
    fn validate_rejects_previous_response_and_conversation() {
        let mut request = ResponseCreateRequest::new("gpt-4.1");
        request.conversation = Some(serde_json::json!({"id":"conv_123"}));
        request.input = Some(serde_json::json!("hello"));
        request.previous_response_id = Some("resp_123".to_owned());

        assert!(request.validate().is_err());
    }

    #[test]
    fn input_tokens_validate_rejects_previous_response_and_conversation() {
        let mut request = ResponseInputTokensRequest::new("gpt-4.1");
        request.conversation = Some(serde_json::json!({"id":"conv_123"}));
        request.previous_response_id = Some("resp_123".to_owned());

        assert!(request.validate().is_err());
    }

    #[test]
    fn response_stream_event_preserves_unknown_fields() {
        let event: ResponseStreamEvent = serde_json::from_str(
            r#"{
                "type":"response.output_text.delta",
                "delta":"hi",
                "obfuscation":"xx"
            }"#,
        )
        .unwrap();

        assert_eq!(event.event_type, "response.output_text.delta");
        assert_eq!(event.extra.get("obfuscation").unwrap(), "xx");
    }

    #[test]
    fn retrieve_stream_query_defaults_stream_true() {
        let query = ResponseRetrieveStreamQuery::default();
        assert!(query.stream);
        assert!(query.starting_after.is_none());
    }

    #[test]
    fn response_create_request_new_sets_model() {
        let request = ResponseCreateRequest::new("gpt-4.1");
        assert_eq!(request.model, "gpt-4.1");
        assert!(request.input.is_none());
    }

    #[test]
    fn response_compact_request_new_sets_model() {
        let request = ResponseCompactRequest::new("gpt-5.1-codex-max");
        assert_eq!(request.model, "gpt-5.1-codex-max");
        assert!(request.input.is_none());
    }
}
