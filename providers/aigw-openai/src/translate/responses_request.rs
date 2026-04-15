//! Request translation: canonical `ChatRequest` → OpenAI Responses API request.
//!
//! Converts the Chat Completions message format into the Responses API's
//! `input` array + `instructions` string. System/developer messages become
//! top-level `instructions`; user/assistant/tool messages become input items.
//!
//! [`ResponsesRequestConfig`] controls behaviour that differs between the
//! public `api.openai.com/v1/responses` endpoint and private backends (e.g.
//! `chatgpt.com/backend-api/codex/responses`).

use std::collections::BTreeMap;

use aigw_core::error::TranslateError;
use aigw_core::model::{
    ChatRequest, MessageContent, ResponseFormat, Role, Tool, ToolCall, ToolChoice,
};
use aigw_core::translate::{RequestTranslator, TranslatedRequest};
use bytes::Bytes;
use http::{HeaderMap, HeaderName, HeaderValue, Method};
use serde_json::{Value, json};

use crate::transport::OpenAITransport;
use crate::wire_types::{
    ResponseCreateRequest, ResponseInput, ResponseTextConfig, ResponseTool, ResponseToolChoice,
    ResponseToolChoiceMode, TypedResponseTool, TypedResponseToolChoice,
};

/// How system/developer messages are mapped into the Responses API request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemHandling {
    /// Extract system/developer messages into the top-level `instructions` field.
    ///
    /// Default behaviour for the public OpenAI Responses API.
    ExtractToInstructions,

    /// Keep system/developer messages inside `input[]` items with role `"developer"`.
    ///
    /// Used by the Codex private backend (`chatgpt.com/backend-api/codex/responses`),
    /// which does NOT accept `instructions` populated from system messages but
    /// requires `instructions` to be present (typically as an empty string).
    MapToDeveloper,
}

/// Controls Responses API translation behaviour.
///
/// The default config targets the public OpenAI Responses API. For private
/// backends like the Codex CLI endpoint, use [`ResponsesRequestConfig::codex`]
/// or build one with the appropriate overrides:
///
/// ```rust,ignore
/// use aigw_openai::{ResponsesRequestConfig, SystemHandling};
///
/// ResponsesRequestConfig {
///     drop_max_tokens: true,
///     default_store: Some(false),
///     default_include: Some(vec!["reasoning.encrypted_content".into()]),
///     default_parallel_tool_calls: Some(true),
///     default_reasoning_summary: Some("auto".into()),
///     force_instructions: true,
///     system_handling: SystemHandling::MapToDeveloper,
///     max_tool_name_len: Some(64),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ResponsesRequestConfig {
    /// How to route system/developer messages.
    pub system_handling: SystemHandling,

    /// Drop `max_tokens` instead of mapping it to `max_output_tokens`.
    ///
    /// The Codex Responses API at `chatgpt.com` rejects `max_output_tokens`
    /// for ChatGPT-backed OAuth sessions.
    pub drop_max_tokens: bool,

    /// Drop `temperature` from the request.
    ///
    /// Codex private backend rejects this field.
    pub drop_temperature: bool,

    /// Drop `top_p` from the request.
    ///
    /// Codex private backend rejects this field.
    pub drop_top_p: bool,

    /// Default value for `store` when the request doesn't specify one.
    ///
    /// Codex private backend requires `store: false`; the public API defaults
    /// to `true` server-side, so `None` means "don't send the field".
    pub default_store: Option<bool>,

    /// Default value for `include` when the request doesn't specify one.
    ///
    /// Codex uses `["reasoning.encrypted_content"]` to request reasoning
    /// model thinking content.
    pub default_include: Option<Vec<String>>,

    /// Default value for `parallel_tool_calls` when the request doesn't specify one.
    pub default_parallel_tool_calls: Option<bool>,

    /// Default value for `reasoning.summary` when the request doesn't specify one.
    ///
    /// Codex uses `"auto"` to enable reasoning summary streaming events.
    pub default_reasoning_summary: Option<String>,

    /// Default value for `reasoning.effort` when the request doesn't specify one.
    ///
    /// Codex defaults to `"medium"`.
    pub default_reasoning_effort: Option<String>,

    /// Emit `instructions: ""` even when there are no system messages.
    ///
    /// The Codex Responses API requires the `instructions` field to be present.
    pub force_instructions: bool,

    /// Maximum tool name length. Names exceeding this are truncated.
    ///
    /// `None` means no truncation (public API). The Codex private backend
    /// enforces a 64-character limit.
    pub max_tool_name_len: Option<usize>,
}

impl Default for ResponsesRequestConfig {
    /// Default config targeting the public OpenAI Responses API.
    fn default() -> Self {
        Self {
            system_handling: SystemHandling::ExtractToInstructions,
            drop_max_tokens: false,
            drop_temperature: false,
            drop_top_p: false,
            default_store: None,
            default_include: None,
            default_parallel_tool_calls: None,
            default_reasoning_summary: None,
            default_reasoning_effort: None,
            force_instructions: false,
            max_tool_name_len: None,
        }
    }
}

impl ResponsesRequestConfig {
    /// Config targeting the Codex private backend (`chatgpt.com/backend-api/codex/responses`).
    ///
    /// Mirrors CLIProxy's Codex behaviour: drops `max_tokens`/`temperature`/`top_p`,
    /// forces `store: false`, injects `include: ["reasoning.encrypted_content"]`,
    /// forces `parallel_tool_calls: true`, sets `reasoning.summary: "auto"` and
    /// defaults `reasoning.effort: "medium"`, keeps system messages as
    /// `developer`-role input items, and truncates tool names to 64 chars.
    #[must_use]
    pub fn codex() -> Self {
        Self {
            system_handling: SystemHandling::MapToDeveloper,
            drop_max_tokens: true,
            drop_temperature: true,
            drop_top_p: true,
            default_store: Some(false),
            default_include: Some(vec!["reasoning.encrypted_content".into()]),
            default_parallel_tool_calls: Some(true),
            default_reasoning_summary: Some("auto".into()),
            default_reasoning_effort: Some("medium".into()),
            force_instructions: true,
            max_tool_name_len: Some(64),
        }
    }
}

/// Translates canonical requests into OpenAI Responses API requests.
pub struct ResponsesRequestTranslator {
    transport: OpenAITransport,
    config: ResponsesRequestConfig,
}

impl ResponsesRequestTranslator {
    pub fn new(transport: OpenAITransport) -> Self {
        Self {
            transport,
            config: ResponsesRequestConfig::default(),
        }
    }

    pub fn with_config(mut self, config: ResponsesRequestConfig) -> Self {
        self.config = config;
        self
    }
}

impl RequestTranslator for ResponsesRequestTranslator {
    fn translate_request(&self, req: &ChatRequest) -> Result<TranslatedRequest, TranslateError> {
        let responses_req = chat_request_to_responses(req, &self.config)?;
        let body = serde_json::to_vec(&responses_req)?;
        let transport_req = self
            .transport
            .prepare_json_request("/responses", &BTreeMap::new());
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
        let mut responses_req = chat_request_to_responses(req, &self.config)?;
        responses_req.stream = Some(true);

        let body = serde_json::to_vec(&responses_req)?;

        let mut extra_headers = BTreeMap::new();
        extra_headers.insert("Accept".to_owned(), "text/event-stream".to_owned());
        let transport_req = self
            .transport
            .prepare_json_request("/responses", &extra_headers);
        let headers = btree_to_headermap(&transport_req.headers)?;

        Ok(TranslatedRequest {
            url: transport_req.url,
            method: Method::POST,
            headers,
            body: Bytes::from(body),
        })
    }
}

/// Core conversion: `ChatRequest` → `ResponseCreateRequest`.
fn chat_request_to_responses(
    req: &ChatRequest,
    config: &ResponsesRequestConfig,
) -> Result<ResponseCreateRequest, TranslateError> {
    let mut instructions_parts: Vec<String> = Vec::new();
    let mut input_items: Vec<Value> = Vec::new();

    for msg in &req.messages {
        match &msg.role {
            Role::System | Role::Developer => match config.system_handling {
                SystemHandling::ExtractToInstructions => {
                    if let Some(text) = extract_text_content(&msg.content) {
                        instructions_parts.push(text);
                    }
                }
                SystemHandling::MapToDeveloper => {
                    let content = translate_message_content(&msg.content, "user");
                    input_items.push(json!({
                        "type": "message",
                        "role": "developer",
                        "content": content,
                    }));
                }
            },
            Role::User => {
                let content = translate_message_content(&msg.content, "user");
                input_items.push(json!({
                    "type": "message",
                    "role": "user",
                    "content": content,
                }));
            }
            Role::Assistant => {
                // Emit text content first (if any), then tool calls.
                if let Some(ref c) = msg.content {
                    let text = extract_text_content(&Some(c.clone())).unwrap_or_default();
                    if !text.is_empty() {
                        let content = translate_message_content(&msg.content, "assistant");
                        input_items.push(json!({
                            "type": "message",
                            "role": "assistant",
                            "content": content,
                        }));
                    }
                }
                if let Some(tool_calls) = &msg.tool_calls {
                    for tc in tool_calls {
                        input_items.push(translate_tool_call(tc));
                    }
                }
            }
            Role::Tool => {
                let output = extract_text_content(&msg.content).unwrap_or_default();
                let call_id = msg.tool_call_id.clone().unwrap_or_default();
                input_items.push(json!({
                    "type": "function_call_output",
                    "call_id": call_id,
                    "output": output,
                }));
            }
            _ => {
                let content = translate_message_content(&msg.content, "user");
                input_items.push(json!({
                    "type": "message",
                    "role": msg.role.to_string(),
                    "content": content,
                }));
            }
        }
    }

    // Instructions: join system messages or apply force_instructions.
    let instructions: Option<Value> = if instructions_parts.is_empty() {
        if config.force_instructions {
            Some(Value::String(String::new()))
        } else {
            None
        }
    } else {
        Some(Value::String(instructions_parts.join("\n\n")))
    };

    let input: Option<ResponseInput> = if input_items.is_empty() {
        None
    } else {
        Some(Value::Array(input_items))
    };

    let tools = req.tools.as_ref().map(|ts| {
        ts.iter()
            .map(|t| translate_tool(t, config))
            .collect::<Vec<ResponseTool>>()
    });

    let tool_choice = req.tool_choice.as_ref().map(translate_tool_choice);

    // Extract Responses-specific fields from `extra` with config fallbacks.
    let store = req
        .extra
        .get("store")
        .and_then(Value::as_bool)
        .or(config.default_store);

    let reasoning = build_reasoning(req, config);

    let include = req
        .extra
        .get("include")
        .and_then(|v| {
            v.as_array()
                .map(|a| a.iter().filter_map(|s| s.as_str().map(String::from)).collect())
        })
        .or_else(|| config.default_include.clone());

    let parallel_tool_calls = req
        .extra
        .get("parallel_tool_calls")
        .and_then(Value::as_bool)
        .or(config.default_parallel_tool_calls);

    let max_output_tokens = if config.drop_max_tokens {
        None
    } else {
        req.max_tokens
    };

    let temperature = if config.drop_temperature {
        None
    } else {
        req.temperature.map(|v| v as f32)
    };

    let top_p = if config.drop_top_p {
        None
    } else {
        req.top_p.map(|v| v as f32)
    };

    let text = translate_response_format(req.response_format.as_ref());

    Ok(ResponseCreateRequest {
        model: req.model.clone(),
        input,
        instructions,
        max_output_tokens,
        temperature,
        top_p,
        tools,
        tool_choice,
        user: req.user.clone(),
        store,
        reasoning,
        include,
        parallel_tool_calls,
        text,
        stream: req.stream,
        background: None,
        conversation: None,
        context_management: None,
        metadata: None,
        previous_response_id: None,
        prompt_cache_key: None,
        prompt_cache_retention: None,
        safety_identifier: None,
        service_tier: None,
        stream_options: None,
        truncation: None,
        extra: Default::default(),
    })
}

/// Build the `reasoning` object from `ChatRequest.extra` fields and config defaults.
///
/// Priority:
/// 1. `extra.reasoning` object (Responses API native form)
/// 2. `extra.reasoning_effort` string (Chat Completions shorthand) → `reasoning.effort`
/// 3. `config.default_reasoning_effort` → `reasoning.effort`
/// 4. `config.default_reasoning_summary` → `reasoning.summary`
fn build_reasoning(req: &ChatRequest, config: &ResponsesRequestConfig) -> Option<Value> {
    let mut obj = serde_json::Map::new();

    // Base from extra.reasoning (nested form).
    if let Some(Value::Object(base)) = req.extra.get("reasoning") {
        for (k, v) in base {
            obj.insert(k.clone(), v.clone());
        }
    }

    // Override effort from reasoning_effort shorthand if not already set.
    if !obj.contains_key("effort")
        && let Some(effort) = req.extra.get("reasoning_effort").and_then(Value::as_str)
    {
        obj.insert("effort".into(), Value::String(effort.to_owned()));
    }

    // Apply config defaults if nothing was provided.
    if !obj.contains_key("effort")
        && let Some(default) = &config.default_reasoning_effort
    {
        obj.insert("effort".into(), Value::String(default.clone()));
    }
    if !obj.contains_key("summary")
        && let Some(default) = &config.default_reasoning_summary
    {
        obj.insert("summary".into(), Value::String(default.clone()));
    }

    if obj.is_empty() {
        None
    } else {
        Some(Value::Object(obj))
    }
}

/// Convert `ChatRequest.response_format` → Responses API `text.format`.
///
/// Chat Completions uses `{ type: "json_schema", json_schema: { name, schema, strict, ... } }`.
/// Responses API flattens this to `{ type: "json_schema", name, schema, strict, ... }`.
fn translate_response_format(rf: Option<&ResponseFormat>) -> Option<ResponseTextConfig> {
    let format_value = match rf? {
        ResponseFormat::Text { .. } => json!({ "type": "text" }),
        ResponseFormat::JsonObject { .. } => json!({ "type": "json_object" }),
        ResponseFormat::JsonSchema { json_schema, .. } => {
            let mut obj = serde_json::Map::new();
            obj.insert("type".into(), Value::String("json_schema".into()));
            obj.insert("name".into(), Value::String(json_schema.name.clone()));
            if let Some(d) = &json_schema.description {
                obj.insert("description".into(), Value::String(d.clone()));
            }
            if let Some(s) = &json_schema.schema {
                obj.insert("schema".into(), s.clone());
            }
            if let Some(strict) = json_schema.strict {
                obj.insert("strict".into(), Value::Bool(strict));
            }
            Value::Object(obj)
        }
    };

    Some(ResponseTextConfig {
        format: Some(format_value),
        verbosity: None,
        extra: Default::default(),
    })
}

/// Extract plain text from `MessageContent`.
fn extract_text_content(content: &Option<MessageContent>) -> Option<String> {
    match content {
        Some(MessageContent::Text(s)) => Some(s.clone()),
        Some(MessageContent::Parts(parts)) => {
            let texts: Vec<&str> = parts
                .iter()
                .filter_map(|p| match p {
                    aigw_core::model::ContentPart::Known(
                        aigw_core::model::TypedContentPart::Text { text, .. },
                    ) => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            if texts.is_empty() {
                None
            } else {
                Some(texts.join(""))
            }
        }
        None => None,
    }
}

/// Convert `MessageContent` to a Responses API `content` value.
///
/// Uses `output_text` for assistant messages and `input_text` for everything else.
fn translate_message_content(content: &Option<MessageContent>, role: &str) -> Value {
    let text_type = if role == "assistant" {
        "output_text"
    } else {
        "input_text"
    };

    match content {
        Some(MessageContent::Text(s)) => {
            json!([{ "type": text_type, "text": s }])
        }
        Some(MessageContent::Parts(parts)) => {
            let items: Vec<Value> = parts
                .iter()
                .map(|p| match p {
                    aigw_core::model::ContentPart::Known(
                        aigw_core::model::TypedContentPart::Text { text, .. },
                    ) => json!({ "type": text_type, "text": text }),
                    aigw_core::model::ContentPart::Known(
                        aigw_core::model::TypedContentPart::ImageUrl { image_url, .. },
                    ) => json!({ "type": "input_image", "image_url": image_url.url }),
                    other => serde_json::to_value(other).unwrap_or(Value::Null),
                })
                .collect();
            Value::Array(items)
        }
        None => json!([]),
    }
}

fn translate_tool_call(tc: &ToolCall) -> Value {
    json!({
        "type": "function_call",
        "call_id": tc.id,
        "name": tc.function.name,
        "arguments": tc.function.arguments,
    })
}

fn translate_tool(tool: &Tool, config: &ResponsesRequestConfig) -> ResponseTool {
    let mut name = tool.function.name.clone();
    if let Some(max_len) = config.max_tool_name_len {
        name.truncate(max_len);
    }

    ResponseTool::Typed(Box::new(TypedResponseTool::Function {
        name,
        description: tool.function.description.clone(),
        parameters: tool.function.parameters.clone(),
        strict: tool.function.strict,
        defer_loading: None,
        extra: Default::default(),
    }))
}

fn translate_tool_choice(tc: &ToolChoice) -> ResponseToolChoice {
    match tc {
        ToolChoice::Mode(mode) => {
            let s = mode.to_string();
            ResponseToolChoice::Mode(ResponseToolChoiceMode::from(s))
        }
        ToolChoice::Named(named) => {
            ResponseToolChoice::Typed(TypedResponseToolChoice::Function {
                name: named.function.name.clone(),
                extra: Default::default(),
            })
        }
        ToolChoice::Raw(obj) => ResponseToolChoice::Raw(obj.clone()),
    }
}

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
        ChatRequest, FunctionCall, FunctionDefinition, Message, MessageContent, Role, Tool,
        ToolCall, ToolChoice, ToolChoiceMode,
    };

    use super::{ResponsesRequestConfig, SystemHandling, chat_request_to_responses};

    fn default_config() -> ResponsesRequestConfig {
        ResponsesRequestConfig::default()
    }

    fn codex_config() -> ResponsesRequestConfig {
        ResponsesRequestConfig::codex()
    }

    fn minimal_request() -> ChatRequest {
        ChatRequest {
            model: "gpt-4.1".into(),
            messages: vec![
                Message {
                    role: Role::System,
                    content: Some(MessageContent::Text("You are helpful.".into())),
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                    extra: Default::default(),
                },
                Message {
                    role: Role::User,
                    content: Some(MessageContent::Text("Hello".into())),
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                    extra: Default::default(),
                },
            ],
            temperature: Some(0.7),
            max_tokens: Some(1024),
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
        }
    }

    #[test]
    fn system_message_becomes_instructions() {
        let resp = chat_request_to_responses(&minimal_request(), &default_config()).unwrap();
        assert_eq!(
            resp.instructions,
            Some(serde_json::Value::String("You are helpful.".into()))
        );
    }

    #[test]
    fn user_message_becomes_input_item() {
        let resp = chat_request_to_responses(&minimal_request(), &default_config()).unwrap();
        let items = resp.input.unwrap();
        let items = items.as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["content"][0]["type"], "input_text");
    }

    #[test]
    fn assistant_content_uses_output_text() {
        let mut req = minimal_request();
        req.messages.push(Message {
            role: Role::Assistant,
            content: Some(MessageContent::Text("Hi there".into())),
            name: None,
            tool_call_id: None,
            tool_calls: None,
            extra: Default::default(),
        });
        let resp = chat_request_to_responses(&req, &default_config()).unwrap();
        let items = resp.input.unwrap();
        let items = items.as_array().unwrap();
        // Last item is the assistant message.
        let last = items.last().unwrap();
        assert_eq!(last["content"][0]["type"], "output_text");
    }

    #[test]
    fn max_tokens_maps_to_max_output_tokens() {
        let resp = chat_request_to_responses(&minimal_request(), &default_config()).unwrap();
        assert_eq!(resp.max_output_tokens, Some(1024));
    }

    #[test]
    fn codex_config_drops_max_tokens() {
        let resp = chat_request_to_responses(&minimal_request(), &codex_config()).unwrap();
        assert_eq!(resp.max_output_tokens, None);
    }

    #[test]
    fn codex_config_sets_default_store() {
        let resp = chat_request_to_responses(&minimal_request(), &codex_config()).unwrap();
        assert_eq!(resp.store, Some(false));
    }

    #[test]
    fn codex_config_sets_default_include() {
        let resp = chat_request_to_responses(&minimal_request(), &codex_config()).unwrap();
        assert_eq!(
            resp.include,
            Some(vec!["reasoning.encrypted_content".into()])
        );
    }

    #[test]
    fn codex_config_forces_empty_instructions() {
        let mut req = minimal_request();
        req.messages.retain(|m| m.role != Role::System);
        let resp = chat_request_to_responses(&req, &codex_config()).unwrap();
        assert_eq!(resp.instructions, Some(serde_json::Value::String("".into())));
    }

    #[test]
    fn default_config_omits_instructions_when_no_system() {
        let mut req = minimal_request();
        req.messages.retain(|m| m.role != Role::System);
        let resp = chat_request_to_responses(&req, &default_config()).unwrap();
        assert_eq!(resp.instructions, None);
    }

    #[test]
    fn codex_config_truncates_tool_names() {
        let mut req = minimal_request();
        req.tools = Some(vec![Tool {
            kind: "function".into(),
            function: FunctionDefinition {
                name: "a".repeat(100),
                description: None,
                parameters: None,
                strict: None,
                extra: Default::default(),
            },
            extra: Default::default(),
        }]);
        let resp = chat_request_to_responses(&req, &codex_config()).unwrap();
        let tools = resp.tools.unwrap();
        let serialized = serde_json::to_value(&tools[0]).unwrap();
        assert_eq!(serialized["name"].as_str().unwrap().len(), 64);
    }

    #[test]
    fn tool_calls_become_function_call_items() {
        let mut req = minimal_request();
        req.messages.push(Message {
            role: Role::Assistant,
            content: None,
            name: None,
            tool_call_id: None,
            tool_calls: Some(vec![ToolCall {
                id: "call_1".into(),
                kind: "function".into(),
                function: FunctionCall {
                    name: "get_weather".into(),
                    arguments: r#"{"location":"SF"}"#.into(),
                    extra: Default::default(),
                },
                extra: Default::default(),
            }]),
            extra: Default::default(),
        });
        req.messages.push(Message {
            role: Role::Tool,
            content: Some(MessageContent::Text("72F sunny".into())),
            name: None,
            tool_call_id: Some("call_1".into()),
            tool_calls: None,
            extra: Default::default(),
        });

        let resp = chat_request_to_responses(&req, &default_config()).unwrap();
        let items = resp.input.unwrap();
        let items = items.as_array().unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[1]["type"], "function_call");
        assert_eq!(items[2]["type"], "function_call_output");
    }

    #[test]
    fn tools_translated_to_responses_format() {
        let mut req = minimal_request();
        req.tools = Some(vec![Tool {
            kind: "function".into(),
            function: FunctionDefinition {
                name: "search".into(),
                description: Some("Search the web".into()),
                parameters: Some(serde_json::json!({"type": "object"})),
                strict: Some(true),
                extra: Default::default(),
            },
            extra: Default::default(),
        }]);
        req.tool_choice = Some(ToolChoice::Mode(ToolChoiceMode::Auto));

        let resp = chat_request_to_responses(&req, &default_config()).unwrap();
        let tools = resp.tools.unwrap();
        let serialized = serde_json::to_value(&tools[0]).unwrap();
        assert_eq!(serialized["type"], "function");
        assert_eq!(serialized["name"], "search");
        assert_eq!(serialized["strict"], true);
    }

    // ── SystemHandling ──────────────────────────────────────────────────

    #[test]
    fn map_to_developer_keeps_system_in_input() {
        let config = ResponsesRequestConfig {
            system_handling: SystemHandling::MapToDeveloper,
            ..Default::default()
        };
        let resp = chat_request_to_responses(&minimal_request(), &config).unwrap();
        let items = resp.input.unwrap();
        let items = items.as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["type"], "message");
        assert_eq!(items[0]["role"], "developer");
        assert_eq!(items[0]["content"][0]["text"], "You are helpful.");
        assert_eq!(items[1]["role"], "user");
        // Instructions is not populated from system in this mode.
        assert_eq!(resp.instructions, None);
    }

    #[test]
    fn codex_config_routes_system_to_developer_and_forces_empty_instructions() {
        let resp = chat_request_to_responses(&minimal_request(), &codex_config()).unwrap();
        let items = resp.input.unwrap();
        let items = items.as_array().unwrap();
        assert_eq!(items[0]["role"], "developer");
        assert_eq!(resp.instructions, Some(serde_json::Value::String("".into())));
    }

    // ── Drop flags ──────────────────────────────────────────────────────

    #[test]
    fn codex_config_drops_temperature_and_top_p() {
        let mut req = minimal_request();
        req.temperature = Some(0.5);
        req.top_p = Some(0.9);
        let resp = chat_request_to_responses(&req, &codex_config()).unwrap();
        assert_eq!(resp.temperature, None);
        assert_eq!(resp.top_p, None);
    }

    #[test]
    fn default_config_forwards_temperature_and_top_p() {
        let mut req = minimal_request();
        req.temperature = Some(0.5);
        req.top_p = Some(0.9);
        let resp = chat_request_to_responses(&req, &default_config()).unwrap();
        assert_eq!(resp.temperature, Some(0.5));
        assert_eq!(resp.top_p, Some(0.9));
    }

    // ── reasoning_effort / reasoning ────────────────────────────────────

    #[test]
    fn reasoning_effort_shorthand_maps_to_reasoning_effort() {
        let mut req = minimal_request();
        req.extra.insert(
            "reasoning_effort".into(),
            serde_json::Value::String("high".into()),
        );
        let resp = chat_request_to_responses(&req, &default_config()).unwrap();
        let reasoning = resp.reasoning.unwrap();
        assert_eq!(reasoning["effort"], "high");
    }

    #[test]
    fn reasoning_object_passes_through() {
        let mut req = minimal_request();
        req.extra.insert(
            "reasoning".into(),
            serde_json::json!({"effort": "low", "summary": "concise"}),
        );
        let resp = chat_request_to_responses(&req, &default_config()).unwrap();
        let reasoning = resp.reasoning.unwrap();
        assert_eq!(reasoning["effort"], "low");
        assert_eq!(reasoning["summary"], "concise");
    }

    #[test]
    fn codex_config_sets_default_reasoning_effort_and_summary() {
        let resp = chat_request_to_responses(&minimal_request(), &codex_config()).unwrap();
        let reasoning = resp.reasoning.unwrap();
        assert_eq!(reasoning["effort"], "medium");
        assert_eq!(reasoning["summary"], "auto");
    }

    #[test]
    fn user_reasoning_effort_overrides_default() {
        let mut req = minimal_request();
        req.extra.insert(
            "reasoning_effort".into(),
            serde_json::Value::String("high".into()),
        );
        let resp = chat_request_to_responses(&req, &codex_config()).unwrap();
        let reasoning = resp.reasoning.unwrap();
        assert_eq!(reasoning["effort"], "high");
        assert_eq!(reasoning["summary"], "auto"); // Default still applied.
    }

    // ── response_format → text.format ──────────────────────────────────

    #[test]
    fn response_format_text_maps_to_text_format() {
        use aigw_core::model::ResponseFormat;
        let mut req = minimal_request();
        req.response_format = Some(ResponseFormat::Text {
            extra: Default::default(),
        });
        let resp = chat_request_to_responses(&req, &default_config()).unwrap();
        let text = resp.text.unwrap();
        assert_eq!(text.format.unwrap()["type"], "text");
    }

    #[test]
    fn response_format_json_object_maps() {
        use aigw_core::model::ResponseFormat;
        let mut req = minimal_request();
        req.response_format = Some(ResponseFormat::JsonObject {
            extra: Default::default(),
        });
        let resp = chat_request_to_responses(&req, &default_config()).unwrap();
        assert_eq!(resp.text.unwrap().format.unwrap()["type"], "json_object");
    }

    #[test]
    fn response_format_json_schema_flattens_fields() {
        use aigw_core::model::{JsonSchema, ResponseFormat};
        let mut req = minimal_request();
        req.response_format = Some(ResponseFormat::JsonSchema {
            json_schema: JsonSchema {
                name: "person".into(),
                description: Some("A person".into()),
                schema: Some(serde_json::json!({"type": "object"})),
                strict: Some(true),
                extra: Default::default(),
            },
            extra: Default::default(),
        });
        let resp = chat_request_to_responses(&req, &default_config()).unwrap();
        let format = resp.text.unwrap().format.unwrap();
        assert_eq!(format["type"], "json_schema");
        assert_eq!(format["name"], "person");
        assert_eq!(format["description"], "A person");
        assert_eq!(format["strict"], true);
        assert_eq!(format["schema"]["type"], "object");
        // Fields must be flattened at top level, not nested under `json_schema`.
        assert!(format.get("json_schema").is_none());
    }

    // ── parallel_tool_calls ─────────────────────────────────────────────

    #[test]
    fn parallel_tool_calls_from_extra() {
        let mut req = minimal_request();
        req.extra
            .insert("parallel_tool_calls".into(), serde_json::Value::Bool(false));
        let resp = chat_request_to_responses(&req, &default_config()).unwrap();
        assert_eq!(resp.parallel_tool_calls, Some(false));
    }

    #[test]
    fn codex_config_forces_parallel_tool_calls_true() {
        let resp = chat_request_to_responses(&minimal_request(), &codex_config()).unwrap();
        assert_eq!(resp.parallel_tool_calls, Some(true));
    }

    #[test]
    fn user_parallel_tool_calls_overrides_default() {
        let mut req = minimal_request();
        req.extra
            .insert("parallel_tool_calls".into(), serde_json::Value::Bool(false));
        let resp = chat_request_to_responses(&req, &codex_config()).unwrap();
        assert_eq!(resp.parallel_tool_calls, Some(false));
    }
}
