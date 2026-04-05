//! Request translation: canonical `ChatRequest` → Anthropic Messages API.
//!
//! Key transformations:
//! - System messages extracted to top-level `system` field
//! - Consecutive tool messages merged into a single user message
//! - Tool definitions unwrapped from OpenAI `function` wrapper
//! - Tool call arguments (JSON string) parsed into JSON objects

use aigw_core::error::TranslateError;
use aigw_core::model::{ChatRequest, ContentPart, Message, MessageContent, Role, TypedContentPart};
use aigw_core::translate::{RequestTranslator, TranslatedRequest};
use bytes::Bytes;
use http::{HeaderMap, Method};

use crate::types::{
    ContentBlock, ImageSource, Message as AnthropicMessage, MessageContent as AnthropicContent,
    MessagesRequest, Metadata, Role as AnthropicRole, SystemPrompt, TypedContentBlock,
};

use super::tools;

const DEFAULT_MAX_TOKENS: u64 = 4096;

/// Translates canonical requests into Anthropic Messages API requests.
pub struct AnthropicRequestTranslator {
    headers: HeaderMap,
    url: String,
    default_max_tokens: u64,
}

impl AnthropicRequestTranslator {
    pub fn new(transport: &crate::Transport, default_max_tokens: Option<u64>) -> Self {
        Self {
            headers: transport.headers().clone(),
            url: transport.url("/v1/messages"),
            default_max_tokens: default_max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
        }
    }
}

impl RequestTranslator for AnthropicRequestTranslator {
    fn translate_request(&self, req: &ChatRequest) -> Result<TranslatedRequest, TranslateError> {
        // Reject unsupported features.
        if let Some(n) = req.n
            && n > 1
        {
            return Err(TranslateError::UnsupportedFeature {
                provider: "anthropic",
                feature: "n > 1".into(),
            });
        }

        let system = extract_system(&req.messages);
        let messages = translate_messages(&req.messages)?;

        let mut extra = serde_json::Map::new();
        // Pass through Anthropic-specific extra fields.
        for (k, v) in &req.extra {
            extra.insert(k.clone(), v.clone());
        }

        // Extract `thinking` from extra if present.
        let thinking = extra
            .remove("thinking")
            .and_then(|v| serde_json::from_value(v).ok());

        let native = MessagesRequest::builder()
            .model(&req.model)
            .messages(messages)
            .max_tokens(req.max_tokens.unwrap_or(self.default_max_tokens))
            .maybe_system(system)
            .maybe_temperature(req.temperature)
            .maybe_top_p(req.top_p)
            .maybe_stop_sequences(req.stop.as_ref().map(|s| s.to_vec()))
            .maybe_stream(req.stream)
            .maybe_tools(req.tools.as_ref().map(|t| tools::translate_tools(t)))
            .maybe_tool_choice(req.tool_choice.as_ref().map(tools::translate_tool_choice))
            .maybe_metadata(req.user.as_ref().map(|u| Metadata {
                user_id: Some(u.clone()),
            }))
            .maybe_thinking(thinking)
            .extra(extra)
            .build();

        let body = serde_json::to_vec(&native)?;

        Ok(TranslatedRequest {
            url: self.url.clone(),
            method: Method::POST,
            headers: self.headers.clone(),
            body: Bytes::from(body),
        })
    }
}

// ─── System message extraction ──────────────────────────────────────────────

/// Extract all system/developer messages and join them into a single `SystemPrompt`.
fn extract_system(messages: &[Message]) -> Option<SystemPrompt> {
    let mut all_texts = Vec::new();

    for msg in messages {
        if !matches!(msg.role, Role::System | Role::Developer) {
            continue;
        }
        match &msg.content {
            Some(MessageContent::Text(s)) => all_texts.push(s.clone()),
            Some(MessageContent::Parts(parts)) => {
                let text: String = parts
                    .iter()
                    .filter_map(|p| match p {
                        ContentPart::Known(TypedContentPart::Text { text, .. }) => {
                            Some(text.as_str())
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");
                if !text.is_empty() {
                    all_texts.push(text);
                }
            }
            None => {}
        }
    }

    if all_texts.is_empty() {
        None
    } else {
        Some(SystemPrompt::Text(all_texts.join("\n\n")))
    }
}

// ─── Message translation ────────────────────────────────────────────────────

/// Translate canonical messages to Anthropic format.
///
/// Filters out system/developer messages (already extracted) and merges
/// consecutive tool messages into a single user message with tool_result blocks.
fn translate_messages(messages: &[Message]) -> Result<Vec<AnthropicMessage>, TranslateError> {
    let non_system: Vec<&Message> = messages
        .iter()
        .filter(|m| !matches!(m.role, Role::System | Role::Developer))
        .collect();

    let mut result = Vec::new();
    let mut i = 0;

    while i < non_system.len() {
        let msg = non_system[i];
        match msg.role {
            Role::User => {
                result.push(translate_user_message(msg)?);
                i += 1;
            }
            Role::Assistant => {
                result.push(translate_assistant_message(msg)?);
                i += 1;
            }
            Role::Tool => {
                // Merge consecutive tool messages into a single user message.
                let mut tool_blocks = Vec::new();
                while i < non_system.len() && non_system[i].role == Role::Tool {
                    tool_blocks.push(translate_tool_result(non_system[i])?);
                    i += 1;
                }
                result.push(AnthropicMessage {
                    role: AnthropicRole::User,
                    content: AnthropicContent::Blocks(tool_blocks),
                });
            }
            _ => {
                // Unknown roles: treat as user.
                result.push(translate_user_message(msg)?);
                i += 1;
            }
        }
    }

    Ok(result)
}

fn translate_user_message(msg: &Message) -> Result<AnthropicMessage, TranslateError> {
    let content = match &msg.content {
        Some(MessageContent::Text(s)) => AnthropicContent::Text(s.clone()),
        Some(MessageContent::Parts(parts)) => {
            let blocks: Result<Vec<_>, _> = parts.iter().map(translate_content_part).collect();
            AnthropicContent::Blocks(blocks?)
        }
        None => AnthropicContent::Text(String::new()),
    };

    Ok(AnthropicMessage {
        role: AnthropicRole::User,
        content,
    })
}

fn translate_assistant_message(msg: &Message) -> Result<AnthropicMessage, TranslateError> {
    let mut blocks = Vec::new();

    // Text content.
    match &msg.content {
        Some(MessageContent::Text(s)) if !s.is_empty() => {
            blocks.push(ContentBlock::Typed(TypedContentBlock::Text {
                text: s.clone(),
                cache_control: None,
            }));
        }
        Some(MessageContent::Parts(parts)) => {
            for part in parts {
                blocks.push(translate_content_part(part)?);
            }
        }
        _ => {}
    }

    // Tool calls → ToolUse blocks.
    if let Some(tool_calls) = &msg.tool_calls {
        for tc in tool_calls {
            let input: serde_json::Value =
                serde_json::from_str(&tc.function.arguments).unwrap_or(serde_json::json!({}));
            blocks.push(ContentBlock::Typed(TypedContentBlock::ToolUse {
                id: tc.id.clone(),
                name: tc.function.name.clone(),
                input,
                cache_control: None,
            }));
        }
    }

    let content = if blocks.is_empty() {
        AnthropicContent::Text(String::new())
    } else {
        AnthropicContent::Blocks(blocks)
    };

    Ok(AnthropicMessage {
        role: AnthropicRole::Assistant,
        content,
    })
}

fn translate_tool_result(msg: &Message) -> Result<ContentBlock, TranslateError> {
    let tool_use_id = msg
        .tool_call_id
        .clone()
        .ok_or(TranslateError::MissingField {
            field: "tool_call_id",
        })?;

    let content = msg.content.as_ref().map(|c| match c {
        MessageContent::Text(s) => crate::types::ToolResultContent::Text(s.clone()),
        MessageContent::Parts(_) => {
            // Serialize parts content as text fallback.
            crate::types::ToolResultContent::Text(serde_json::to_string(c).unwrap_or_default())
        }
    });

    Ok(ContentBlock::Typed(TypedContentBlock::ToolResult {
        tool_use_id,
        content,
        is_error: None,
        cache_control: None,
    }))
}

fn translate_content_part(part: &ContentPart) -> Result<ContentBlock, TranslateError> {
    match part {
        ContentPart::Known(TypedContentPart::Text { text, .. }) => {
            Ok(ContentBlock::Typed(TypedContentBlock::Text {
                text: text.clone(),
                cache_control: None,
            }))
        }
        ContentPart::Known(TypedContentPart::ImageUrl { image_url, .. }) => {
            let source = translate_image_source(&image_url.url)?;
            Ok(ContentBlock::Typed(TypedContentBlock::Image {
                source,
                cache_control: None,
            }))
        }
        ContentPart::Raw(obj) => {
            // Both sides are now serde_json::Map — direct clone.
            Ok(ContentBlock::Raw(obj.clone()))
        }
        _ => Err(TranslateError::IncompatibleContent {
            reason: "unsupported content part type for Anthropic".into(),
        }),
    }
}

fn translate_image_source(url: &str) -> Result<ImageSource, TranslateError> {
    if let Some(rest) = url.strip_prefix("data:") {
        let (header, data) =
            rest.split_once(',')
                .ok_or_else(|| TranslateError::IncompatibleContent {
                    reason: "malformed data: URI".into(),
                })?;
        let media_type =
            header
                .strip_suffix(";base64")
                .ok_or_else(|| TranslateError::IncompatibleContent {
                    reason: "data: URI must be base64-encoded".into(),
                })?;
        Ok(ImageSource::Base64 {
            media_type: media_type.to_owned(),
            data: data.to_owned(),
        })
    } else {
        Ok(ImageSource::Url {
            url: url.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aigw_core::model::{FunctionCall, ImageUrl, ToolCall};

    fn user_msg(text: &str) -> Message {
        Message {
            role: Role::User,
            content: Some(MessageContent::Text(text.into())),
            name: None,
            tool_call_id: None,
            tool_calls: None,
            extra: Default::default(),
        }
    }

    fn system_msg(text: &str) -> Message {
        Message {
            role: Role::System,
            content: Some(MessageContent::Text(text.into())),
            name: None,
            tool_call_id: None,
            tool_calls: None,
            extra: Default::default(),
        }
    }

    fn tool_msg(tool_call_id: &str, content: &str) -> Message {
        Message {
            role: Role::Tool,
            content: Some(MessageContent::Text(content.into())),
            name: None,
            tool_call_id: Some(tool_call_id.into()),
            tool_calls: None,
            extra: Default::default(),
        }
    }

    #[test]
    fn extract_no_system() {
        let msgs = vec![user_msg("hi")];
        assert!(extract_system(&msgs).is_none());
    }

    #[test]
    fn extract_single_system() {
        let msgs = vec![system_msg("You are helpful"), user_msg("hi")];
        match extract_system(&msgs) {
            Some(SystemPrompt::Text(s)) => assert_eq!(s, "You are helpful"),
            other => panic!("expected Text, got {other:?}"),
        }
    }

    #[test]
    fn extract_multiple_system_messages() {
        let msgs = vec![
            system_msg("You are helpful"),
            system_msg("Be concise"),
            user_msg("hi"),
        ];
        match extract_system(&msgs) {
            Some(SystemPrompt::Text(s)) => assert_eq!(s, "You are helpful\n\nBe concise"),
            other => panic!("expected Text, got {other:?}"),
        }
    }

    #[test]
    fn translate_messages_filters_system() {
        let msgs = vec![system_msg("system"), user_msg("hello")];
        let result = translate_messages(&msgs).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, AnthropicRole::User);
    }

    #[test]
    fn consecutive_tool_messages_merged() {
        let msgs = vec![
            user_msg("check weather"),
            // assistant with tool call would be here in real conversation
            tool_msg("call_1", "72F sunny"),
            tool_msg("call_2", "65F cloudy"),
        ];
        let result = translate_messages(&msgs).unwrap();
        // user + merged tool results (as single user message)
        assert_eq!(result.len(), 2);
        assert_eq!(result[1].role, AnthropicRole::User);
        match &result[1].content {
            AnthropicContent::Blocks(blocks) => {
                assert_eq!(blocks.len(), 2);
                // Both should be ToolResult blocks.
                for block in blocks {
                    assert!(matches!(
                        block,
                        ContentBlock::Typed(TypedContentBlock::ToolResult { .. })
                    ));
                }
            }
            _ => panic!("expected Blocks"),
        }
    }

    #[test]
    fn assistant_message_with_tool_calls() {
        let msg = Message {
            role: Role::Assistant,
            content: Some(MessageContent::Text("Let me check.".into())),
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
        };

        let result = translate_assistant_message(&msg).unwrap();
        assert_eq!(result.role, AnthropicRole::Assistant);
        match &result.content {
            AnthropicContent::Blocks(blocks) => {
                assert_eq!(blocks.len(), 2); // text + tool_use
                assert!(matches!(
                    &blocks[0],
                    ContentBlock::Typed(TypedContentBlock::Text { text, .. }) if text == "Let me check."
                ));
                assert!(matches!(
                    &blocks[1],
                    ContentBlock::Typed(TypedContentBlock::ToolUse { name, .. }) if name == "get_weather"
                ));
            }
            _ => panic!("expected Blocks"),
        }
    }

    #[test]
    fn image_url_translation() {
        let source = translate_image_source("https://example.com/img.png").unwrap();
        assert!(matches!(source, ImageSource::Url { url } if url == "https://example.com/img.png"));
    }

    #[test]
    fn image_data_uri_translation() {
        let source = translate_image_source("data:image/png;base64,iVBOR...").unwrap();
        match source {
            ImageSource::Base64 { media_type, data } => {
                assert_eq!(media_type, "image/png");
                assert_eq!(data, "iVBOR...");
            }
            _ => panic!("expected Base64"),
        }
    }

    #[test]
    fn image_content_in_user_message() {
        let msg = Message {
            role: Role::User,
            content: Some(MessageContent::Parts(vec![
                ContentPart::Known(TypedContentPart::Text {
                    text: "What's in this image?".into(),
                    extra: Default::default(),
                }),
                ContentPart::Known(TypedContentPart::ImageUrl {
                    image_url: ImageUrl {
                        url: "https://example.com/cat.jpg".into(),
                        detail: None,
                        extra: Default::default(),
                    },
                    extra: Default::default(),
                }),
            ])),
            name: None,
            tool_call_id: None,
            tool_calls: None,
            extra: Default::default(),
        };

        let result = translate_user_message(&msg).unwrap();
        match &result.content {
            AnthropicContent::Blocks(blocks) => {
                assert_eq!(blocks.len(), 2);
                assert!(matches!(
                    &blocks[0],
                    ContentBlock::Typed(TypedContentBlock::Text { .. })
                ));
                assert!(matches!(
                    &blocks[1],
                    ContentBlock::Typed(TypedContentBlock::Image { .. })
                ));
            }
            _ => panic!("expected Blocks"),
        }
    }

    #[test]
    fn tool_result_missing_tool_call_id() {
        let msg = Message {
            role: Role::Tool,
            content: Some(MessageContent::Text("result".into())),
            name: None,
            tool_call_id: None, // missing!
            tool_calls: None,
            extra: Default::default(),
        };

        let err = translate_tool_result(&msg).unwrap_err();
        assert!(matches!(
            err,
            TranslateError::MissingField {
                field: "tool_call_id"
            }
        ));
    }
}
