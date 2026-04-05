//! Canonical request types.

use bon::Builder;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ForwardCompatible, JsonObject, OneOrMany, json_object_is_empty};

// ─── ChatRequest ────────────────────────────────────────────────────────────

/// Canonical chat completion request.
///
/// Field names match the OpenAI Chat Completions API so that inbound requests
/// deserialize without transformation. Provider translators consume this type
/// and produce provider-native requests.
///
/// Fields not explicitly modeled (e.g. `logprobs`, `service_tier`, `reasoning_effort`)
/// land in `extra` via `#[serde(flatten)]` and are available to translators that
/// understand them.
#[derive(Debug, Clone, Builder, Serialize, Deserialize)]
#[builder(on(String, into))]
pub struct ChatRequest {
    /// Model identifier (e.g. `"gpt-4.1"`, `"claude-sonnet-4-20250514"`, `"gemini-2.5-pro"`).
    pub model: String,

    /// Conversation messages.
    pub messages: Vec<Message>,

    /// Sampling temperature (0.0–2.0). Provider-specific ranges may differ.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Maximum number of tokens to generate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,

    /// Nucleus sampling parameter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,

    /// Stop sequences. OpenAI accepts a single string or an array.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop: Option<StopSequence>,

    /// Whether to stream the response via SSE.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Available tools for the model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,

    /// Tool selection strategy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,

    /// Structured output format.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,

    /// Frequency penalty (-2.0–2.0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,

    /// Presence penalty (-2.0–2.0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,

    /// Number of completions to generate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,

    /// Deterministic sampling seed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,

    /// End-user identifier for abuse detection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,

    /// All other fields — provider-specific parameters pass through untouched.
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    #[builder(default)]
    pub extra: JsonObject,
}

// ─── Message ────────────────────────────────────────────────────────────────

/// A conversation message.
///
/// Follows the OpenAI flat structure: `content`, `tool_calls`, and `tool_call_id`
/// are sibling fields. Translators for Anthropic/Gemini restructure these into
/// their native formats (content blocks, function parts, etc.).
#[derive(Debug, Clone, Builder, Serialize, Deserialize)]
#[builder(on(String, into))]
pub struct Message {
    /// Message role.
    pub role: Role,

    /// Text content — plain string or array of content parts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<MessageContent>,

    /// Participant name (for multi-participant conversations).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Tool call ID — present on `role: "tool"` messages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,

    /// Tool calls — present on `role: "assistant"` messages that invoke tools.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,

    /// Pass-through fields.
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    #[builder(default)]
    pub extra: JsonObject,
}

// ─── Role ───────────────────────────────────────────────────────────────────

/// Message role.
///
/// Covers all roles across providers: `system`/`developer` (OpenAI),
/// `user`, `assistant`, `tool` (OpenAI/Anthropic/Gemini).
#[derive(
    Debug, Clone, PartialEq, Eq, Hash,
    Serialize, Deserialize,
    strum::Display, strum::EnumString, strum::AsRefStr,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Role {
    /// System-level instruction.
    System,
    /// OpenAI "developer" role (functionally equivalent to system).
    Developer,
    /// User message.
    User,
    /// Assistant / model response.
    Assistant,
    /// Tool result message.
    Tool,
    /// Forward-compatible catch-all.
    #[serde(untagged)]
    #[strum(default)]
    Unknown(String),
}

// ─── Content ────────────────────────────────────────────────────────────────

/// Message content — either a plain string or an array of typed content parts.
///
/// Both OpenAI and Anthropic accept this duality.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// Plain text content.
    Text(String),
    /// Array of typed content parts (text, image, audio, etc.).
    Parts(Vec<ContentPart>),
}

/// A content part within a multipart message.
///
/// Known types are strongly typed via [`TypedContentPart`]; unknown types
/// fall back to raw JSON for forward compatibility.
pub type ContentPart = ForwardCompatible<TypedContentPart>;

/// Strongly-typed content part variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TypedContentPart {
    /// Text content.
    Text {
        text: String,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    /// Image via URL.
    ImageUrl {
        image_url: ImageUrl,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    /// Audio input.
    InputAudio {
        input_audio: Value,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    /// File attachment.
    File {
        file: Value,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
}

/// Image URL reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    /// The image URL (can be an `https://` URL or a `data:` URI).
    pub url: String,
    /// Image detail level (`"auto"`, `"low"`, `"high"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

// ─── Tools ──────────────────────────────────────────────────────────────────

/// Tool definition (OpenAI function-calling format).
///
/// Translators for Anthropic unwrap the `function` layer and rename
/// `parameters` → `input_schema`. Gemini translators wrap into
/// `functionDeclarations`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Tool type — currently always `"function"`.
    #[serde(rename = "type")]
    pub kind: String,
    /// Function definition.
    pub function: FunctionDefinition,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

/// Function definition within a [`Tool`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    /// Function name.
    pub name: String,
    /// Human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// JSON Schema for parameters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Value>,
    /// Whether to enable strict schema adherence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

/// A tool call emitted by the assistant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique tool call ID.
    pub id: String,
    /// Tool type — currently always `"function"`.
    #[serde(rename = "type")]
    pub kind: String,
    /// The function call details.
    pub function: FunctionCall,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

/// Function call details within a [`ToolCall`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Function name.
    pub name: String,
    /// JSON-encoded arguments string.
    pub arguments: String,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

// ─── Tool Choice ────────────────────────────────────────────────────────────

/// Tool selection strategy.
///
/// OpenAI accepts a string (`"auto"`, `"none"`, `"required"`) or an object
/// (`{ "type": "function", "function": { "name": "X" } }`). This enum handles
/// both forms via custom serde.
#[derive(Debug, Clone, PartialEq)]
pub enum ToolChoice {
    /// String mode: `"none"`, `"auto"`, `"required"`.
    Mode(ToolChoiceMode),
    /// Force a specific function by name.
    Named(NamedToolChoice),
    /// Unknown format — preserved as raw JSON.
    Raw(JsonObject),
}

#[derive(
    Debug, Clone, PartialEq, Eq,
    Serialize, Deserialize,
    strum::Display, strum::EnumString, strum::AsRefStr,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ToolChoiceMode {
    None,
    Auto,
    Required,
    #[serde(untagged)]
    #[strum(default)]
    Unknown(String),
}

/// Named tool choice — force calling a specific function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamedToolChoice {
    #[serde(rename = "type")]
    pub kind: String,
    pub function: NamedToolChoiceFunction,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamedToolChoiceFunction {
    pub name: String,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

impl Serialize for ToolChoice {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Mode(mode) => mode.serialize(serializer),
            Self::Named(named) => named.serialize(serializer),
            Self::Raw(raw) => raw.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for ToolChoice {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        match &value {
            Value::String(s) => {
                // strum's FromStr handles known variants + Unknown fallback
                let mode: ToolChoiceMode = s
                    .parse()
                    .expect("strum default variant guarantees infallible parse");
                Ok(Self::Mode(mode))
            }
            Value::Object(_) => {
                match serde_json::from_value::<NamedToolChoice>(value.clone()) {
                    Ok(named) => Ok(Self::Named(named)),
                    Err(_) => {
                        let obj: JsonObject = serde_json::from_value(value)
                            .map_err(serde::de::Error::custom)?;
                        Ok(Self::Raw(obj))
                    }
                }
            }
            _ => Err(serde::de::Error::custom(
                "tool_choice must be a string or object",
            )),
        }
    }
}

// ─── Response Format ────────────────────────────────────────────────────────

/// Structured output response format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseFormat {
    /// Plain text (default).
    Text {
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    /// JSON object mode.
    JsonObject {
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
    /// JSON with a specific schema.
    JsonSchema {
        json_schema: JsonSchema,
        #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
        extra: JsonObject,
    },
}

/// JSON Schema definition for structured output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchema {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
    #[serde(flatten, default, skip_serializing_if = "json_object_is_empty")]
    pub extra: JsonObject,
}

// ─── Stop Sequence ──────────────────────────────────────────────────────────

/// Stop sequences — OpenAI accepts either a single string or an array.
pub type StopSequence = OneOrMany<String>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_minimal_request() {
        let json = r#"{
            "model": "gpt-4.1",
            "messages": [
                { "role": "user", "content": "Hello" }
            ]
        }"#;

        let req: ChatRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.model, "gpt-4.1");
        assert_eq!(req.messages.len(), 1);
        assert_eq!(req.messages[0].role, Role::User);
    }

    #[test]
    fn extra_fields_preserved_via_flatten() {
        let json = r#"{
            "model": "gpt-4.1",
            "messages": [],
            "logprobs": true,
            "reasoning_effort": "high"
        }"#;

        let req: ChatRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.extra.get("logprobs").unwrap(), &Value::Bool(true));
        assert_eq!(req.extra.get("reasoning_effort").unwrap(), "high");

        // Round-trips back
        let reserialized = serde_json::to_value(&req).unwrap();
        assert_eq!(reserialized["logprobs"], true);
        assert_eq!(reserialized["reasoning_effort"], "high");
    }

    #[test]
    fn multipart_content_with_unknown_part() {
        let json = r#"{
            "model": "gpt-4.1",
            "messages": [{
                "role": "user",
                "content": [
                    { "type": "text", "text": "describe this" },
                    { "type": "image_url", "image_url": { "url": "https://example.com/img.png" } },
                    { "type": "mystery", "payload": 42 }
                ]
            }]
        }"#;

        let req: ChatRequest = serde_json::from_str(json).unwrap();
        let content = req.messages[0].content.as_ref().unwrap();
        match content {
            MessageContent::Parts(parts) => {
                assert!(matches!(parts[0], ContentPart::Known(TypedContentPart::Text { .. })));
                assert!(matches!(parts[1], ContentPart::Known(TypedContentPart::ImageUrl { .. })));
                assert!(matches!(parts[2], ContentPart::Raw(..)));
            }
            MessageContent::Text(_) => panic!("expected multipart"),
        }
    }

    #[test]
    fn tool_choice_string_mode() {
        let tc: ToolChoice = serde_json::from_str(r#""required""#).unwrap();
        assert_eq!(tc, ToolChoice::Mode(ToolChoiceMode::Required));
    }

    #[test]
    fn tool_choice_named_round_trips() {
        let json = r#"{"type":"function","function":{"name":"get_weather"}}"#;
        let tc: ToolChoice = serde_json::from_str(json).unwrap();
        match &tc {
            ToolChoice::Named(n) => assert_eq!(n.function.name, "get_weather"),
            _ => panic!("expected Named"),
        }
        let reserialized = serde_json::to_string(&tc).unwrap();
        assert!(reserialized.contains("get_weather"));
    }

    #[test]
    fn stop_sequence_one_or_many() {
        let one: StopSequence = serde_json::from_str(r#""END""#).unwrap();
        assert_eq!(one.into_vec(), vec!["END"]);

        let many: StopSequence = serde_json::from_str(r#"["STOP", "END"]"#).unwrap();
        assert_eq!(many.into_vec(), vec!["STOP", "END"]);
    }

    #[test]
    fn unknown_role_round_trips() {
        let role: Role = serde_json::from_str(r#""critic""#).unwrap();
        assert_eq!(role, Role::Unknown("critic".into()));
        assert_eq!(serde_json::to_string(&role).unwrap(), r#""critic""#);
    }

    #[test]
    fn assistant_message_with_tool_calls() {
        let json = r#"{
            "role": "assistant",
            "content": "Let me check the weather.",
            "tool_calls": [{
                "id": "call_abc",
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "arguments": "{\"location\":\"SF\"}"
                }
            }]
        }"#;

        let msg: Message = serde_json::from_str(json).unwrap();
        assert_eq!(msg.role, Role::Assistant);
        assert!(msg.tool_calls.is_some());
        let tc = &msg.tool_calls.unwrap()[0];
        assert_eq!(tc.function.name, "get_weather");
    }

    #[test]
    fn tool_result_message() {
        let json = r#"{
            "role": "tool",
            "tool_call_id": "call_abc",
            "content": "72°F, sunny"
        }"#;

        let msg: Message = serde_json::from_str(json).unwrap();
        assert_eq!(msg.role, Role::Tool);
        assert_eq!(msg.tool_call_id.as_deref(), Some("call_abc"));
    }
}
