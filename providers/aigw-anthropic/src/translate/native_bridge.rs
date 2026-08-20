//! Native-protocol bridge — the inverse direction of
//! [`AnthropicRequestTranslator`] / [`AnthropicResponseTranslator`].
//!
//! The request/response translators take a canonical [`ChatRequest`] and emit
//! Anthropic-native wire types (outbound: the gateway acting as an Anthropic
//! *client*). This module goes the other way, so a gateway can *serve* clients
//! that speak the Anthropic Messages API (`POST /v1/messages`) while routing to
//! a non-Anthropic backend (e.g. an OpenAI Chat Completions upstream). This is
//! what lets a bare OpenAI-compatible key back a Claude Code session.
//!
//! Three helpers cover the three directions:
//!
//! - [`messages_request_to_canonical`] — Anthropic-native [`MessagesRequest`] →
//!   canonical [`ChatRequest`]. Used by the receiving end of a `/v1/messages`
//!   HTTP call.
//! - [`chat_response_to_messages`] — canonical [`ChatResponse`] →
//!   Anthropic-native [`MessagesResponse`]. Used to format a non-streaming
//!   response back to the client.
//! - [`stream_event_to_anthropic_sse`] — single canonical [`StreamEvent`] →
//!   zero or more Anthropic SSE frames ([`AnthropicSseFrame`]). Stateful via
//!   [`SseContext`] because Anthropic's block-lifecycle wire model
//!   (`message_start` → `content_block_start`/`delta`/`stop` →
//!   `message_delta` → `message_stop`) is more structured than the flat
//!   canonical event stream, so a single canonical event may open/close blocks
//!   and emit several frames.
//!
//! [`ChatRequest`]: aigw_core::model::ChatRequest
//! [`ChatResponse`]: aigw_core::model::ChatResponse
//! [`StreamEvent`]: aigw_core::model::StreamEvent
//! [`MessagesRequest`]: crate::types::MessagesRequest
//! [`MessagesResponse`]: crate::types::MessagesResponse
//! [`AnthropicRequestTranslator`]: super::request::AnthropicRequestTranslator
//! [`AnthropicResponseTranslator`]: super::response::AnthropicResponseTranslator

use std::collections::HashMap;

use aigw_core::OneOrMany;
use aigw_core::error::TranslateError;
use aigw_core::model::{
    ChatRequest, ChatResponse, ContentPart, FinishReason as CanonicalFinishReason,
    FunctionDefinition, ImageUrl, Message, MessageContent, NamedToolChoice,
    NamedToolChoiceFunction, Role, StopSequence, StreamEvent, ThinkingRequest, ThinkingSource,
    Tool as CanonicalTool, ToolCall, ToolChoice, ToolChoiceMode, TypedContentPart, Usage,
};
use serde_json::{Value, json};

use crate::types::{
    ContentBlock, ImageSource, MessageContent as AnthropicContent, MessagesRequest,
    MessagesResponse, Role as AnthropicRole, StopReason, SystemPrompt, ThinkingConfig,
    Tool as AnthropicTool, ToolChoice as AnthropicToolChoice, ToolResultContent, TypedContentBlock,
    Usage as AnthropicUsage,
};

use super::tools::tool_use_to_canonical;

// ─── Anthropic → canonical request ──────────────────────────────────────────

/// Convert an Anthropic-native [`MessagesRequest`] into a canonical
/// [`ChatRequest`].
///
/// Used by gateways that accept Anthropic-native traffic and forward it to a
/// non-Anthropic backend. The inverse of
/// [`AnthropicRequestTranslator::translate_request`](super::request::AnthropicRequestTranslator).
///
/// Key transformations (mirror of the outbound translator):
/// - Top-level `system` → a leading `role: "system"` canonical message.
/// - Anthropic `tool_result` blocks on a `user` turn → standalone canonical
///   `role: "tool"` messages (OpenAI shape).
/// - `tool_use` blocks on an `assistant` turn → `tool_calls`.
/// - `thinking` / `redacted_thinking` blocks → canonical thinking parts tagged
///   with [`ThinkingSource::Anthropic`] so their signatures round-trip.
///
/// # Errors
///
/// Currently always returns `Ok` — the conversion is total. The `Result` is
/// kept for forward-compatibility with future validation.
pub fn messages_request_to_canonical(req: MessagesRequest) -> Result<ChatRequest, TranslateError> {
    let mut messages: Vec<Message> = Vec::new();

    // System prompt → a leading System message.
    if let Some(system) = &req.system {
        let text = system_prompt_to_text(system);
        if !text.is_empty() {
            messages.push(text_message(Role::System, text));
        }
    }

    // Walk conversation turns.
    for msg in req.messages {
        let role = msg.role;
        match msg.content {
            AnthropicContent::Text(text) => {
                messages.push(text_message(canonical_role(role), text));
            }
            AnthropicContent::Blocks(blocks) => match role {
                AnthropicRole::Assistant => {
                    let (content, tool_calls) = split_assistant_blocks(blocks);
                    messages.push(Message {
                        role: Role::Assistant,
                        content,
                        name: None,
                        tool_call_id: None,
                        tool_calls,
                        extra: Default::default(),
                    });
                }
                AnthropicRole::User => {
                    // tool_result blocks become standalone tool messages;
                    // everything else stays on the user turn.
                    let (tool_results, other) = partition_tool_results(blocks);
                    for tr in tool_results {
                        messages.push(tr);
                    }
                    if let Some(content) = user_blocks_to_content(other) {
                        messages.push(Message {
                            role: Role::User,
                            content: Some(content),
                            name: None,
                            tool_call_id: None,
                            tool_calls: None,
                            extra: Default::default(),
                        });
                    }
                }
                AnthropicRole::System => {
                    if let Some(content) = user_blocks_to_content(blocks) {
                        messages.push(Message {
                            role: Role::System,
                            content: Some(content),
                            name: None,
                            tool_call_id: None,
                            tool_calls: None,
                            extra: Default::default(),
                        });
                    }
                }
            },
        }
    }

    let tools = req
        .tools
        .map(|tools| tools.into_iter().map(anthropic_tool_to_canonical).collect());
    let tool_choice = req.tool_choice.map(anthropic_tool_choice_to_canonical);
    let stop = req.stop_sequences.and_then(stop_sequences_to_canonical);
    let thinking = req.thinking.map(anthropic_thinking_to_canonical);
    let user = req.metadata.and_then(|m| m.user_id);

    // Anthropic `top_k` has no canonical field — preserve it in `extra` so a
    // top-k-aware backend can still consume it.
    let mut extra = req.extra;
    if let Some(top_k) = req.top_k {
        extra.insert("top_k".to_owned(), Value::from(top_k));
    }

    Ok(ChatRequest::builder()
        .model(req.model)
        .messages(messages)
        .max_tokens(req.max_tokens)
        .maybe_temperature(req.temperature)
        .maybe_top_p(req.top_p)
        .maybe_stop(stop)
        .maybe_stream(req.stream)
        .maybe_tools(tools)
        .maybe_tool_choice(tool_choice)
        .maybe_thinking(thinking)
        .maybe_user(user)
        .extra(extra)
        .build())
}

fn text_message(role: Role, text: String) -> Message {
    Message {
        role,
        content: Some(MessageContent::Text(text)),
        name: None,
        tool_call_id: None,
        tool_calls: None,
        extra: Default::default(),
    }
}

const fn canonical_role(role: AnthropicRole) -> Role {
    match role {
        AnthropicRole::User => Role::User,
        AnthropicRole::Assistant => Role::Assistant,
        AnthropicRole::System => Role::System,
    }
}

fn system_prompt_to_text(system: &SystemPrompt) -> String {
    match system {
        SystemPrompt::Text(s) => s.clone(),
        SystemPrompt::Blocks(blocks) => blocks
            .iter()
            .map(|b| b.text.as_str())
            .collect::<Vec<_>>()
            .join("\n\n"),
    }
}

fn stop_sequences_to_canonical(seqs: Vec<String>) -> Option<StopSequence> {
    match seqs.len() {
        0 => None,
        1 => Some(OneOrMany::One(seqs.into_iter().next().unwrap_or_default())),
        _ => Some(OneOrMany::Many(seqs)),
    }
}

/// Split an assistant turn's blocks into `(content, tool_calls)`.
///
/// Block order is preserved. A turn whose only content parts are text
/// collapses to a plain `MessageContent::Text`; anything richer (thinking,
/// image, raw) keeps the multipart form so replay fidelity holds.
fn split_assistant_blocks(
    blocks: Vec<ContentBlock>,
) -> (Option<MessageContent>, Option<Vec<ToolCall>>) {
    let mut parts: Vec<ContentPart> = Vec::new();
    let mut tool_calls: Vec<ToolCall> = Vec::new();
    let mut only_text = true;

    for block in blocks {
        match block {
            ContentBlock::Typed(TypedContentBlock::Text { text, .. }) => {
                parts.push(text_part(text));
            }
            ContentBlock::Typed(TypedContentBlock::Thinking {
                thinking,
                signature,
            }) => {
                only_text = false;
                parts.push(thinking_part(thinking, signature));
            }
            ContentBlock::Typed(TypedContentBlock::RedactedThinking { data }) => {
                only_text = false;
                parts.push(ContentPart::Known(TypedContentPart::RedactedThinking {
                    data,
                    source: Some(ThinkingSource::Anthropic),
                    extra: Default::default(),
                }));
            }
            ContentBlock::Typed(TypedContentBlock::ToolUse {
                id, name, input, ..
            }) => {
                tool_calls.push(tool_use_to_canonical(&id, &name, &input));
            }
            ContentBlock::Typed(TypedContentBlock::Image { source, .. }) => {
                only_text = false;
                parts.push(image_part(&source));
            }
            // tool_result never appears on an assistant turn — ignore.
            ContentBlock::Typed(TypedContentBlock::ToolResult { .. }) => {}
            ContentBlock::Raw(obj) => {
                only_text = false;
                parts.push(ContentPart::Raw(obj));
            }
        }
    }

    let content = collapse_parts(parts, only_text);
    let tool_calls = (!tool_calls.is_empty()).then_some(tool_calls);
    (content, tool_calls)
}

/// Split a user turn's blocks into `(tool_result_messages, remaining_blocks)`.
fn partition_tool_results(blocks: Vec<ContentBlock>) -> (Vec<Message>, Vec<ContentBlock>) {
    let mut tool_results = Vec::new();
    let mut other = Vec::new();
    for block in blocks {
        match block {
            ContentBlock::Typed(TypedContentBlock::ToolResult {
                tool_use_id,
                content,
                ..
            }) => {
                tool_results.push(Message {
                    role: Role::Tool,
                    content: Some(MessageContent::Text(tool_result_content_to_string(content))),
                    name: None,
                    tool_call_id: Some(tool_use_id),
                    tool_calls: None,
                    extra: Default::default(),
                });
            }
            other_block => other.push(other_block),
        }
    }
    (tool_results, other)
}

fn tool_result_content_to_string(content: Option<ToolResultContent>) -> String {
    match content {
        None => String::new(),
        Some(ToolResultContent::Text(s)) => s,
        Some(ToolResultContent::Blocks(blocks)) => blocks
            .iter()
            .map(|b| match b {
                ContentBlock::Typed(TypedContentBlock::Text { text, .. }) => text.clone(),
                // The OpenAI tool role carries only text — an image tool result
                // (e.g. Claude Code's Read on an image) can't be forwarded, so
                // emit a clean placeholder rather than raw JSON.
                ContentBlock::Typed(TypedContentBlock::Image { .. }) => "[image]".to_owned(),
                other => serde_json::to_string(other).unwrap_or_default(),
            })
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

fn user_blocks_to_content(blocks: Vec<ContentBlock>) -> Option<MessageContent> {
    let mut parts: Vec<ContentPart> = Vec::new();
    let mut only_text = true;
    for block in blocks {
        match block {
            ContentBlock::Typed(TypedContentBlock::Text { text, .. }) => {
                parts.push(text_part(text))
            }
            ContentBlock::Typed(TypedContentBlock::Image { source, .. }) => {
                only_text = false;
                parts.push(image_part(&source));
            }
            ContentBlock::Raw(obj) => {
                only_text = false;
                parts.push(ContentPart::Raw(obj));
            }
            // thinking / tool_use / tool_result don't belong on a user turn.
            ContentBlock::Typed(_) => {}
        }
    }
    collapse_parts(parts, only_text)
}

/// Collapse content parts: `None` if empty, a plain `Text` if every part is
/// text, otherwise the multipart form.
fn collapse_parts(parts: Vec<ContentPart>, only_text: bool) -> Option<MessageContent> {
    if parts.is_empty() {
        None
    } else if only_text {
        let joined: String = parts
            .iter()
            .filter_map(|p| match p {
                ContentPart::Known(TypedContentPart::Text { text, .. }) => Some(text.as_str()),
                _ => None,
            })
            .collect();
        Some(MessageContent::Text(joined))
    } else {
        Some(MessageContent::Parts(parts))
    }
}

fn text_part(text: String) -> ContentPart {
    ContentPart::Known(TypedContentPart::Text {
        text,
        extra: Default::default(),
    })
}

fn thinking_part(thinking: String, signature: String) -> ContentPart {
    ContentPart::Known(TypedContentPart::Thinking {
        thinking,
        signature,
        source: Some(ThinkingSource::Anthropic),
        extra: Default::default(),
    })
}

fn image_part(source: &ImageSource) -> ContentPart {
    ContentPart::Known(TypedContentPart::ImageUrl {
        image_url: ImageUrl {
            url: image_source_to_url(source),
            detail: None,
            extra: Default::default(),
        },
        extra: Default::default(),
    })
}

fn image_source_to_url(source: &ImageSource) -> String {
    match source {
        ImageSource::Base64 { media_type, data } => {
            format!("data:{media_type};base64,{data}")
        }
        ImageSource::Url { url } => url.clone(),
    }
}

fn anthropic_tool_to_canonical(tool: AnthropicTool) -> CanonicalTool {
    CanonicalTool {
        kind: "function".to_owned(),
        function: FunctionDefinition {
            name: tool.name,
            description: tool.description,
            parameters: Some(tool.input_schema),
            strict: None,
            extra: Default::default(),
        },
        extra: Default::default(),
    }
}

fn anthropic_tool_choice_to_canonical(tc: AnthropicToolChoice) -> ToolChoice {
    match tc {
        AnthropicToolChoice::Auto { .. } => ToolChoice::Mode(ToolChoiceMode::Auto),
        AnthropicToolChoice::Any { .. } => ToolChoice::Mode(ToolChoiceMode::Required),
        AnthropicToolChoice::None { .. } => ToolChoice::Mode(ToolChoiceMode::None),
        AnthropicToolChoice::Tool { name, .. } => ToolChoice::Named(NamedToolChoice {
            kind: "function".to_owned(),
            function: NamedToolChoiceFunction {
                name,
                extra: Default::default(),
            },
            extra: Default::default(),
        }),
    }
}

fn anthropic_thinking_to_canonical(thinking: ThinkingConfig) -> ThinkingRequest {
    match thinking {
        ThinkingConfig::Enabled { budget_tokens } => ThinkingRequest::Budget {
            budget_tokens: u32::try_from(budget_tokens).unwrap_or(u32::MAX),
        },
        ThinkingConfig::Adaptive => ThinkingRequest::Auto,
        ThinkingConfig::Disabled => ThinkingRequest::Disabled,
    }
}

// ─── Canonical → Anthropic response ─────────────────────────────────────────

/// Convert a canonical [`ChatResponse`] into an Anthropic-native
/// [`MessagesResponse`].
///
/// Anthropic returns a single message, so only the first choice is used.
///
/// # Errors
///
/// Currently always returns `Ok`. Reserved for forward-compatibility.
pub fn chat_response_to_messages(resp: ChatResponse) -> Result<MessagesResponse, TranslateError> {
    let model = resp.model;
    let id = resp.id;
    let usage = resp
        .usage
        .map(canonical_usage_to_anthropic)
        .unwrap_or(AnthropicUsage {
            input_tokens: 0,
            output_tokens: 0,
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
        });

    let choice = resp.choices.into_iter().next();
    let (content, stop_reason) = match choice {
        Some(choice) => (
            assistant_message_to_blocks(&choice.message),
            choice.finish_reason.map(StopReason::from),
        ),
        None => (Vec::new(), None),
    };

    Ok(MessagesResponse {
        id,
        r#type: "message".to_owned(),
        role: AnthropicRole::Assistant,
        content,
        model,
        stop_reason,
        stop_sequence: None,
        usage,
    })
}

fn assistant_message_to_blocks(msg: &Message) -> Vec<ContentBlock> {
    let mut blocks: Vec<ContentBlock> = Vec::new();

    match &msg.content {
        Some(MessageContent::Text(s)) if !s.is_empty() => {
            blocks.push(text_block(s.clone()));
        }
        Some(MessageContent::Parts(parts)) => {
            for part in parts {
                if let Some(block) = content_part_to_block(part) {
                    blocks.push(block);
                }
            }
        }
        _ => {}
    }

    if let Some(tool_calls) = &msg.tool_calls {
        for tc in tool_calls {
            let input: Value =
                serde_json::from_str(&tc.function.arguments).unwrap_or_else(|_| json!({}));
            blocks.push(ContentBlock::Typed(TypedContentBlock::ToolUse {
                id: tc.id.clone(),
                name: tc.function.name.clone(),
                input,
                cache_control: None,
            }));
        }
    }

    blocks
}

fn content_part_to_block(part: &ContentPart) -> Option<ContentBlock> {
    match part {
        ContentPart::Known(TypedContentPart::Text { text, .. }) => Some(text_block(text.clone())),
        ContentPart::Known(TypedContentPart::Thinking {
            thinking,
            signature,
            ..
        }) => Some(ContentBlock::Typed(TypedContentBlock::Thinking {
            thinking: thinking.clone(),
            signature: signature.clone(),
        })),
        ContentPart::Known(TypedContentPart::RedactedThinking { data, .. }) => {
            Some(ContentBlock::Typed(TypedContentBlock::RedactedThinking {
                data: data.clone(),
            }))
        }
        ContentPart::Known(TypedContentPart::ImageUrl { image_url, .. }) => {
            Some(ContentBlock::Typed(TypedContentBlock::Image {
                source: url_to_image_source(&image_url.url),
                cache_control: None,
            }))
        }
        ContentPart::Raw(obj) => Some(ContentBlock::Raw(obj.clone())),
        // Audio / File have no Anthropic response representation — drop.
        ContentPart::Known(_) => None,
    }
}

fn text_block(text: String) -> ContentBlock {
    ContentBlock::Typed(TypedContentBlock::Text {
        text,
        cache_control: None,
    })
}

fn url_to_image_source(url: &str) -> ImageSource {
    if let Some(rest) = url.strip_prefix("data:")
        && let Some((header, data)) = rest.split_once(',')
        && let Some(media_type) = header.strip_suffix(";base64")
    {
        return ImageSource::Base64 {
            media_type: media_type.to_owned(),
            data: data.to_owned(),
        };
    }
    ImageSource::Url {
        url: url.to_owned(),
    }
}

fn canonical_usage_to_anthropic(usage: Usage) -> AnthropicUsage {
    AnthropicUsage {
        input_tokens: usage.prompt_tokens.unwrap_or(0),
        output_tokens: usage.completion_tokens.unwrap_or(0),
        cache_creation_input_tokens: usage
            .extra
            .get("cache_creation_input_tokens")
            .and_then(Value::as_u64),
        cache_read_input_tokens: usage
            .extra
            .get("cache_read_input_tokens")
            .and_then(Value::as_u64),
    }
}

// ─── Canonical stream → Anthropic SSE ───────────────────────────────────────

/// A single Anthropic SSE frame: a named `event:` plus its `data:` JSON.
///
/// Anthropic's wire protocol names every SSE event (`message_start`,
/// `content_block_delta`, …) *and* repeats the name in the JSON `type` field.
/// Rendering is done by [`AnthropicSseFrame::to_sse_bytes`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnthropicSseFrame {
    /// The SSE `event:` name.
    pub event: String,
    /// The SSE `data:` payload (a JSON object string).
    pub data: String,
}

impl AnthropicSseFrame {
    /// Render this frame to on-the-wire SSE bytes (`event: …\ndata: …\n\n`).
    #[must_use]
    pub fn to_sse_bytes(&self) -> Vec<u8> {
        format!("event: {}\ndata: {}\n\n", self.event, self.data).into_bytes()
    }

    fn new(event: &str, data: Value) -> Self {
        Self {
            event: event.to_owned(),
            data: data.to_string(),
        }
    }
}

/// The content block currently open in the Anthropic output stream.
///
/// Anthropic allows only one open block at a time; the wrapped `usize` is that
/// block's index in the response `content` array.
#[derive(Debug, Clone, Copy)]
enum OpenBlock {
    Text(usize),
    Thinking(usize),
    Tool(usize),
}

impl OpenBlock {
    const fn index(self) -> usize {
        match self {
            Self::Text(i) | Self::Thinking(i) | Self::Tool(i) => i,
        }
    }
}

/// Per-stream state for the canonical → Anthropic-native SSE bridge.
///
/// Anthropic's protocol is block-structured: a `message_start` opens the
/// response, each content block is bracketed by `content_block_start` /
/// `content_block_stop`, and `message_delta` + `message_stop` close it. The
/// canonical stream is flatter, so this context tracks which block is open, the
/// next block index to assign, and the mapping from canonical tool-call indices
/// to Anthropic block indices. Finish reason and usage are buffered because
/// OpenAI-style upstreams report them *after* the finish signal, whereas
/// Anthropic emits them together at the end.
#[derive(Debug, Default)]
pub struct SseContext {
    /// Model name surfaced in `message_start`.
    pub model: String,
    /// Message id surfaced in `message_start` (from the first `ResponseMeta`).
    pub message_id: String,
    /// When `true`, `ResponseMeta` won't overwrite `model` — used to echo the
    /// client's requested model rather than the upstream's.
    model_pinned: bool,
    started: bool,
    stopped: bool,
    next_index: usize,
    open: Option<OpenBlock>,
    tool_block_index: HashMap<u32, usize>,
    finish_reason: Option<CanonicalFinishReason>,
    usage: Option<Usage>,
}

impl SseContext {
    /// Build a context with a fixed fallback model name, used when the
    /// canonical stream's `ResponseMeta` doesn't carry one. `ResponseMeta` may
    /// still overwrite it with the upstream model.
    #[must_use]
    pub fn with_model(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            ..Default::default()
        }
    }

    /// Build a context whose model is **pinned** — surfaced in `message_start`
    /// and never overwritten by `ResponseMeta`. Use to echo the client's
    /// requested model instead of the upstream's.
    #[must_use]
    pub fn with_pinned_model(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            model_pinned: true,
            ..Default::default()
        }
    }

    fn ensure_started(&mut self, out: &mut Vec<AnthropicSseFrame>) {
        if self.started {
            return;
        }
        self.started = true;
        out.push(AnthropicSseFrame::new(
            "message_start",
            json!({
                "type": "message_start",
                "message": {
                    "id": self.message_id,
                    "type": "message",
                    "role": "assistant",
                    "model": self.model,
                    "content": [],
                    "stop_reason": Value::Null,
                    "stop_sequence": Value::Null,
                    "usage": { "input_tokens": 0, "output_tokens": 0 },
                }
            }),
        ));
    }

    fn close_open(&mut self, out: &mut Vec<AnthropicSseFrame>) {
        if let Some(block) = self.open.take() {
            out.push(AnthropicSseFrame::new(
                "content_block_stop",
                json!({ "type": "content_block_stop", "index": block.index() }),
            ));
        }
    }

    fn ensure_text_block(&mut self, out: &mut Vec<AnthropicSseFrame>) -> usize {
        if let Some(OpenBlock::Text(idx)) = self.open {
            return idx;
        }
        self.close_open(out);
        let idx = self.alloc_index();
        self.open = Some(OpenBlock::Text(idx));
        out.push(AnthropicSseFrame::new(
            "content_block_start",
            json!({
                "type": "content_block_start",
                "index": idx,
                "content_block": { "type": "text", "text": "" },
            }),
        ));
        idx
    }

    fn ensure_thinking_block(&mut self, out: &mut Vec<AnthropicSseFrame>) -> usize {
        if let Some(OpenBlock::Thinking(idx)) = self.open {
            return idx;
        }
        self.close_open(out);
        let idx = self.alloc_index();
        self.open = Some(OpenBlock::Thinking(idx));
        out.push(AnthropicSseFrame::new(
            "content_block_start",
            json!({
                "type": "content_block_start",
                "index": idx,
                "content_block": { "type": "thinking", "thinking": "", "signature": "" },
            }),
        ));
        idx
    }

    fn alloc_index(&mut self) -> usize {
        let idx = self.next_index;
        self.next_index += 1;
        idx
    }

    fn emit_message_delta_and_stop(&mut self, out: &mut Vec<AnthropicSseFrame>) {
        if self.stopped {
            return;
        }
        self.stopped = true;
        let stop_reason = self
            .finish_reason
            .clone()
            .map_or(StopReason::EndTurn, StopReason::from);
        out.push(AnthropicSseFrame::new(
            "message_delta",
            json!({
                "type": "message_delta",
                "delta": { "stop_reason": stop_reason, "stop_sequence": Value::Null },
                // The terminal `message_delta` carries the full, real usage.
                // OpenAI-style upstreams only report token counts at
                // end-of-stream — too late for `message_start` — but Anthropic
                // clients (Claude Code) read `input_tokens` from here too, so
                // this is where accurate accounting lands.
                "usage": streaming_usage_json(self.usage.as_ref()),
            }),
        ));
        out.push(AnthropicSseFrame::new(
            "message_stop",
            json!({ "type": "message_stop" }),
        ));
    }
}

/// Build the Anthropic streaming `usage` object from canonical usage.
///
/// Always carries `input_tokens` + `output_tokens`, plus
/// `cache_read_input_tokens` / `cache_creation_input_tokens` when the upstream
/// reported them (OpenAI Chat surfaces cache hits as
/// `prompt_tokens_details.cached_tokens`; the Responses API as
/// `input_tokens_details`).
fn streaming_usage_json(usage: Option<&Usage>) -> Value {
    let mut obj = serde_json::Map::new();
    let (input, output) = usage.map_or((0, 0), |u| {
        (
            u.prompt_tokens.unwrap_or(0),
            u.completion_tokens.unwrap_or(0),
        )
    });
    obj.insert("input_tokens".to_owned(), Value::from(input));
    obj.insert("output_tokens".to_owned(), Value::from(output));
    if let Some(usage) = usage {
        if let Some(read) = cache_read_tokens(usage) {
            obj.insert("cache_read_input_tokens".to_owned(), Value::from(read));
        }
        if let Some(creation) = usage
            .extra
            .get("cache_creation_input_tokens")
            .and_then(Value::as_u64)
        {
            obj.insert(
                "cache_creation_input_tokens".to_owned(),
                Value::from(creation),
            );
        }
    }
    Value::Object(obj)
}

/// Extract cache-read (cache-hit) input tokens from canonical usage, checking
/// the Anthropic-style key first, then OpenAI's `prompt_tokens_details`.
fn cache_read_tokens(usage: &Usage) -> Option<u64> {
    usage
        .extra
        .get("cache_read_input_tokens")
        .and_then(Value::as_u64)
        .or_else(|| {
            usage
                .extra
                .get("prompt_tokens_details")
                .and_then(|d| d.get("cached_tokens"))
                .and_then(Value::as_u64)
        })
}

/// Convert a single canonical [`StreamEvent`] into zero or more Anthropic SSE
/// frames, advancing `ctx`.
///
/// The mapping is one-to-many: a `ContentDelta` may first open a text block
/// (`content_block_start`) before emitting the delta; `Done` closes any open
/// block and emits `message_delta` + `message_stop`. Empty deltas produce no
/// frames.
#[must_use]
pub fn stream_event_to_anthropic_sse(
    event: &StreamEvent,
    ctx: &mut SseContext,
) -> Vec<AnthropicSseFrame> {
    let mut out = Vec::new();
    match event {
        StreamEvent::ResponseMeta { id, model } => {
            ctx.message_id = id.clone();
            if !ctx.model_pinned && !model.is_empty() {
                ctx.model = model.clone();
            }
            ctx.ensure_started(&mut out);
        }

        StreamEvent::ContentDelta(text) if !text.is_empty() => {
            ctx.ensure_started(&mut out);
            let idx = ctx.ensure_text_block(&mut out);
            out.push(text_delta_frame(idx, text));
        }
        StreamEvent::ContentDelta(_) => {}

        StreamEvent::ReasoningStart { .. } => {
            ctx.ensure_started(&mut out);
            ctx.ensure_thinking_block(&mut out);
        }
        StreamEvent::ReasoningDelta(text) if !text.is_empty() => {
            ctx.ensure_started(&mut out);
            let idx = ctx.ensure_thinking_block(&mut out);
            out.push(thinking_delta_frame(idx, text));
        }
        StreamEvent::ReasoningDelta(_) => {}
        StreamEvent::ReasoningEnd { signature, .. } => {
            if let Some(OpenBlock::Thinking(idx)) = ctx.open {
                if !signature.is_empty() {
                    out.push(signature_delta_frame(idx, signature));
                }
                ctx.close_open(&mut out);
            }
        }
        #[expect(deprecated, reason = "bridge legacy parsers still emitting it")]
        StreamEvent::ReasoningSignature(signature) => {
            if let Some(OpenBlock::Thinking(idx)) = ctx.open
                && !signature.is_empty()
            {
                out.push(signature_delta_frame(idx, signature));
            }
        }

        StreamEvent::ToolCallStart { index, id, name } => {
            ctx.ensure_started(&mut out);
            ctx.close_open(&mut out);
            let idx = ctx.alloc_index();
            ctx.tool_block_index.insert(*index, idx);
            ctx.open = Some(OpenBlock::Tool(idx));
            out.push(AnthropicSseFrame::new(
                "content_block_start",
                json!({
                    "type": "content_block_start",
                    "index": idx,
                    "content_block": { "type": "tool_use", "id": id, "name": name, "input": {} },
                }),
            ));
        }
        StreamEvent::ToolCallDelta { index, arguments } if !arguments.is_empty() => {
            if let Some(&idx) = ctx.tool_block_index.get(index) {
                out.push(input_json_delta_frame(idx, arguments));
            }
        }
        StreamEvent::ToolCallDelta { .. } => {}

        StreamEvent::Finish(reason) => {
            ctx.finish_reason = Some(reason.clone());
            ctx.close_open(&mut out);
        }
        StreamEvent::Usage(usage) => {
            ctx.usage = Some(usage.clone());
        }
        StreamEvent::Done => {
            ctx.ensure_started(&mut out);
            ctx.close_open(&mut out);
            ctx.emit_message_delta_and_stop(&mut out);
        }
    }
    out
}

fn text_delta_frame(index: usize, text: &str) -> AnthropicSseFrame {
    AnthropicSseFrame::new(
        "content_block_delta",
        json!({
            "type": "content_block_delta",
            "index": index,
            "delta": { "type": "text_delta", "text": text },
        }),
    )
}

fn thinking_delta_frame(index: usize, thinking: &str) -> AnthropicSseFrame {
    AnthropicSseFrame::new(
        "content_block_delta",
        json!({
            "type": "content_block_delta",
            "index": index,
            "delta": { "type": "thinking_delta", "thinking": thinking },
        }),
    )
}

fn signature_delta_frame(index: usize, signature: &str) -> AnthropicSseFrame {
    AnthropicSseFrame::new(
        "content_block_delta",
        json!({
            "type": "content_block_delta",
            "index": index,
            "delta": { "type": "signature_delta", "signature": signature },
        }),
    )
}

fn input_json_delta_frame(index: usize, partial_json: &str) -> AnthropicSseFrame {
    AnthropicSseFrame::new(
        "content_block_delta",
        json!({
            "type": "content_block_delta",
            "index": index,
            "delta": { "type": "input_json_delta", "partial_json": partial_json },
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Message as AnthropicMessage;
    use aigw_core::model::{Choice, FunctionCall};

    // ── messages_request_to_canonical ────────────────────────────────────

    fn user_text(text: &str) -> AnthropicMessage {
        AnthropicMessage {
            role: AnthropicRole::User,
            content: AnthropicContent::Text(text.to_owned()),
        }
    }

    #[test]
    fn request_user_text_round_trips() {
        let req = MessagesRequest::builder()
            .model("claude-sonnet-4-20250514")
            .messages(vec![user_text("Hello")])
            .max_tokens(1024)
            .build();
        let canonical = messages_request_to_canonical(req).unwrap();
        assert_eq!(canonical.model, "claude-sonnet-4-20250514");
        assert_eq!(canonical.max_tokens, Some(1024));
        assert_eq!(canonical.messages.len(), 1);
        assert_eq!(canonical.messages[0].role, Role::User);
        assert!(matches!(
            &canonical.messages[0].content,
            Some(MessageContent::Text(s)) if s == "Hello"
        ));
    }

    #[test]
    fn request_system_becomes_leading_system_message() {
        let req = MessagesRequest::builder()
            .model("claude-sonnet-4-20250514")
            .messages(vec![user_text("hi")])
            .max_tokens(64)
            .system(SystemPrompt::Text("You are helpful.".to_owned()))
            .build();
        let canonical = messages_request_to_canonical(req).unwrap();
        assert_eq!(canonical.messages[0].role, Role::System);
        assert!(matches!(
            &canonical.messages[0].content,
            Some(MessageContent::Text(s)) if s == "You are helpful."
        ));
    }

    #[test]
    fn request_assistant_tool_use_becomes_tool_calls() {
        let req = MessagesRequest::builder()
            .model("m")
            .max_tokens(64)
            .messages(vec![AnthropicMessage {
                role: AnthropicRole::Assistant,
                content: AnthropicContent::Blocks(vec![
                    ContentBlock::Typed(TypedContentBlock::Text {
                        text: "Let me check.".to_owned(),
                        cache_control: None,
                    }),
                    ContentBlock::Typed(TypedContentBlock::ToolUse {
                        id: "toolu_1".to_owned(),
                        name: "get_weather".to_owned(),
                        input: json!({ "location": "SF" }),
                        cache_control: None,
                    }),
                ]),
            }])
            .build();
        let canonical = messages_request_to_canonical(req).unwrap();
        let msg = &canonical.messages[0];
        assert_eq!(msg.role, Role::Assistant);
        assert!(matches!(&msg.content, Some(MessageContent::Text(s)) if s == "Let me check."));
        let tcs = msg.tool_calls.as_ref().unwrap();
        assert_eq!(tcs[0].id, "toolu_1");
        assert_eq!(tcs[0].function.name, "get_weather");
        assert_eq!(tcs[0].function.arguments, r#"{"location":"SF"}"#);
    }

    #[test]
    fn request_tool_result_becomes_tool_message() {
        let req = MessagesRequest::builder()
            .model("m")
            .max_tokens(64)
            .messages(vec![AnthropicMessage {
                role: AnthropicRole::User,
                content: AnthropicContent::Blocks(vec![
                    ContentBlock::Typed(TypedContentBlock::ToolResult {
                        tool_use_id: "toolu_1".to_owned(),
                        content: Some(ToolResultContent::Text("72F sunny".to_owned())),
                        is_error: None,
                        cache_control: None,
                    }),
                    ContentBlock::Typed(TypedContentBlock::Text {
                        text: "thanks".to_owned(),
                        cache_control: None,
                    }),
                ]),
            }])
            .build();
        let canonical = messages_request_to_canonical(req).unwrap();
        // tool_result becomes a standalone tool message, text stays on user.
        assert_eq!(canonical.messages.len(), 2);
        assert_eq!(canonical.messages[0].role, Role::Tool);
        assert_eq!(
            canonical.messages[0].tool_call_id.as_deref(),
            Some("toolu_1")
        );
        assert!(matches!(
            &canonical.messages[0].content,
            Some(MessageContent::Text(s)) if s == "72F sunny"
        ));
        assert_eq!(canonical.messages[1].role, Role::User);
        assert!(matches!(
            &canonical.messages[1].content,
            Some(MessageContent::Text(s)) if s == "thanks"
        ));
    }

    #[test]
    fn tool_result_with_image_block_uses_placeholder() {
        let req = MessagesRequest::builder()
            .model("m")
            .max_tokens(64)
            .messages(vec![AnthropicMessage {
                role: AnthropicRole::User,
                content: AnthropicContent::Blocks(vec![ContentBlock::Typed(
                    TypedContentBlock::ToolResult {
                        tool_use_id: "toolu_1".to_owned(),
                        content: Some(ToolResultContent::Blocks(vec![
                            ContentBlock::Typed(TypedContentBlock::Text {
                                text: "see image:".to_owned(),
                                cache_control: None,
                            }),
                            ContentBlock::Typed(TypedContentBlock::Image {
                                source: ImageSource::Base64 {
                                    media_type: "image/png".to_owned(),
                                    data: "aW1n".to_owned(),
                                },
                                cache_control: None,
                            }),
                        ])),
                        is_error: None,
                        cache_control: None,
                    },
                )]),
            }])
            .build();
        let canonical = messages_request_to_canonical(req).unwrap();
        assert_eq!(canonical.messages[0].role, Role::Tool);
        assert!(matches!(
            &canonical.messages[0].content,
            Some(MessageContent::Text(s)) if s == "see image:\n[image]"
        ));
    }

    #[test]
    fn request_image_becomes_image_url_part() {
        let req = MessagesRequest::builder()
            .model("m")
            .max_tokens(64)
            .messages(vec![AnthropicMessage {
                role: AnthropicRole::User,
                content: AnthropicContent::Blocks(vec![
                    ContentBlock::Typed(TypedContentBlock::Text {
                        text: "describe".to_owned(),
                        cache_control: None,
                    }),
                    ContentBlock::Typed(TypedContentBlock::Image {
                        source: ImageSource::Base64 {
                            media_type: "image/png".to_owned(),
                            data: "aW1n".to_owned(),
                        },
                        cache_control: None,
                    }),
                ]),
            }])
            .build();
        let canonical = messages_request_to_canonical(req).unwrap();
        let parts = match canonical.messages[0].content.as_ref().unwrap() {
            MessageContent::Parts(p) => p,
            other => panic!("expected Parts, got {other:?}"),
        };
        assert!(matches!(
            &parts[1],
            ContentPart::Known(TypedContentPart::ImageUrl { image_url, .. })
                if image_url.url == "data:image/png;base64,aW1n"
        ));
    }

    #[test]
    fn request_tools_and_tool_choice_map() {
        let req = MessagesRequest::builder()
            .model("m")
            .max_tokens(64)
            .messages(vec![user_text("go")])
            .tools(vec![AnthropicTool {
                name: "get_weather".to_owned(),
                description: Some("weather".to_owned()),
                input_schema: json!({ "type": "object" }),
                cache_control: None,
            }])
            .tool_choice(AnthropicToolChoice::Tool {
                name: "get_weather".to_owned(),
                disable_parallel_tool_use: None,
            })
            .build();
        let canonical = messages_request_to_canonical(req).unwrap();
        let tool = &canonical.tools.as_ref().unwrap()[0];
        assert_eq!(tool.kind, "function");
        assert_eq!(tool.function.name, "get_weather");
        assert!(matches!(
            canonical.tool_choice,
            Some(ToolChoice::Named(n)) if n.function.name == "get_weather"
        ));
    }

    #[test]
    fn request_thinking_config_maps() {
        let req = MessagesRequest::builder()
            .model("m")
            .max_tokens(64)
            .messages(vec![user_text("go")])
            .thinking(ThinkingConfig::Enabled {
                budget_tokens: 5000,
            })
            .build();
        let canonical = messages_request_to_canonical(req).unwrap();
        assert_eq!(
            canonical.thinking,
            Some(ThinkingRequest::Budget {
                budget_tokens: 5000
            })
        );
    }

    #[test]
    fn request_deserializes_from_wire_json() {
        // The whole point of the inbound bridge: parse a raw Anthropic body.
        let json = r#"{
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 1024,
            "system": "be brief",
            "messages": [ { "role": "user", "content": "hi" } ]
        }"#;
        let req: MessagesRequest = serde_json::from_str(json).unwrap();
        let canonical = messages_request_to_canonical(req).unwrap();
        assert_eq!(canonical.messages[0].role, Role::System);
        assert_eq!(canonical.messages[1].role, Role::User);
    }

    #[test]
    fn request_deserializes_inline_system_messages() {
        let json = r#"{
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 1024,
            "system": "top-level instructions",
            "messages": [
                { "role": "system", "content": "inline instructions" },
                {
                    "role": "system",
                    "content": [{ "type": "text", "text": "block instructions" }]
                },
                { "role": "user", "content": "hi" }
            ]
        }"#;
        let req: MessagesRequest = serde_json::from_str(json).unwrap();
        let canonical = messages_request_to_canonical(req).unwrap();

        assert_eq!(
            canonical
                .messages
                .iter()
                .map(|message| message.role.clone())
                .collect::<Vec<_>>(),
            [Role::System, Role::System, Role::System, Role::User]
        );
        assert!(matches!(
            &canonical.messages[1].content,
            Some(MessageContent::Text(text)) if text == "inline instructions"
        ));
        assert!(matches!(
            &canonical.messages[2].content,
            Some(MessageContent::Text(text)) if text == "block instructions"
        ));
    }

    // ── chat_response_to_messages ────────────────────────────────────────

    fn assistant_choice(msg: Message, finish: CanonicalFinishReason) -> Choice {
        Choice {
            index: 0,
            message: msg,
            finish_reason: Some(finish),
            extra: Default::default(),
        }
    }

    #[test]
    fn response_text_becomes_message() {
        let resp = ChatResponse {
            id: "chatcmpl-x".to_owned(),
            object: "chat.completion".to_owned(),
            created: 0,
            model: "gpt-4.1".to_owned(),
            choices: vec![assistant_choice(
                Message {
                    role: Role::Assistant,
                    content: Some(MessageContent::Text("Hi there!".to_owned())),
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                    extra: Default::default(),
                },
                CanonicalFinishReason::Stop,
            )],
            usage: Some(Usage {
                prompt_tokens: Some(8),
                completion_tokens: Some(4),
                total_tokens: Some(12),
                extra: Default::default(),
            }),
            extra: Default::default(),
        };
        let out = chat_response_to_messages(resp).unwrap();
        assert_eq!(out.id, "chatcmpl-x");
        assert_eq!(out.model, "gpt-4.1");
        assert_eq!(out.role, AnthropicRole::Assistant);
        assert_eq!(out.stop_reason, Some(StopReason::EndTurn));
        assert_eq!(out.usage.input_tokens, 8);
        assert_eq!(out.usage.output_tokens, 4);
        assert!(matches!(
            &out.content[0],
            ContentBlock::Typed(TypedContentBlock::Text { text, .. }) if text == "Hi there!"
        ));
    }

    #[test]
    fn response_tool_calls_become_tool_use_blocks() {
        let resp = ChatResponse {
            id: "id".to_owned(),
            object: "chat.completion".to_owned(),
            created: 0,
            model: "m".to_owned(),
            choices: vec![assistant_choice(
                Message {
                    role: Role::Assistant,
                    content: None,
                    name: None,
                    tool_call_id: None,
                    tool_calls: Some(vec![ToolCall {
                        id: "call_1".to_owned(),
                        kind: "function".to_owned(),
                        function: FunctionCall {
                            name: "get_weather".to_owned(),
                            arguments: r#"{"location":"SF"}"#.to_owned(),
                            extra: Default::default(),
                        },
                        extra: Default::default(),
                    }]),
                    extra: Default::default(),
                },
                CanonicalFinishReason::ToolCalls,
            )],
            usage: None,
            extra: Default::default(),
        };
        let out = chat_response_to_messages(resp).unwrap();
        assert_eq!(out.stop_reason, Some(StopReason::ToolUse));
        match &out.content[0] {
            ContentBlock::Typed(TypedContentBlock::ToolUse {
                id, name, input, ..
            }) => {
                assert_eq!(id, "call_1");
                assert_eq!(name, "get_weather");
                assert_eq!(input["location"], "SF");
            }
            other => panic!("expected tool_use, got {other:?}"),
        }
    }

    // ── stream_event_to_anthropic_sse ────────────────────────────────────

    fn data(frame: &AnthropicSseFrame) -> Value {
        serde_json::from_str(&frame.data).unwrap()
    }

    /// Advance the bridge state, discarding the emitted frames — used to
    /// prime a context before the frames a test actually asserts on.
    fn feed(ctx: &mut SseContext, ev: StreamEvent) {
        let _ = stream_event_to_anthropic_sse(&ev, ctx);
    }

    #[test]
    fn response_meta_emits_message_start() {
        let mut ctx = SseContext::default();
        let frames = stream_event_to_anthropic_sse(
            &StreamEvent::ResponseMeta {
                id: "msg_1".into(),
                model: "gpt-4.1".into(),
            },
            &mut ctx,
        );
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].event, "message_start");
        let v = data(&frames[0]);
        assert_eq!(v["message"]["id"], "msg_1");
        assert_eq!(v["message"]["model"], "gpt-4.1");
    }

    #[test]
    fn pinned_model_survives_response_meta() {
        let mut ctx = SseContext::with_pinned_model("claude-sonnet-4-5");
        let frames = stream_event_to_anthropic_sse(
            &StreamEvent::ResponseMeta {
                id: "resp_1".into(),
                model: "gpt-4.1".into(),
            },
            &mut ctx,
        );
        // message_start keeps the pinned model, and takes the upstream message id.
        assert_eq!(data(&frames[0])["message"]["model"], "claude-sonnet-4-5");
        assert_eq!(data(&frames[0])["message"]["id"], "resp_1");
    }

    #[test]
    fn content_delta_opens_text_block_then_delta() {
        let mut ctx = SseContext::with_model("m");
        feed(
            &mut ctx,
            StreamEvent::ResponseMeta {
                id: "msg_1".into(),
                model: String::new(),
            },
        );
        let frames =
            stream_event_to_anthropic_sse(&StreamEvent::ContentDelta("Hello".into()), &mut ctx);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].event, "content_block_start");
        assert_eq!(data(&frames[0])["content_block"]["type"], "text");
        assert_eq!(frames[1].event, "content_block_delta");
        assert_eq!(data(&frames[1])["delta"]["text"], "Hello");

        // A second delta reuses the open block (no new start).
        let frames =
            stream_event_to_anthropic_sse(&StreamEvent::ContentDelta(" world".into()), &mut ctx);
        assert_eq!(frames.len(), 1);
        assert_eq!(data(&frames[0])["delta"]["text"], " world");
    }

    #[test]
    fn thinking_block_with_signature() {
        let mut ctx = SseContext::with_model("m");
        feed(
            &mut ctx,
            StreamEvent::ResponseMeta {
                id: "msg_1".into(),
                model: String::new(),
            },
        );
        let frames = stream_event_to_anthropic_sse(
            &StreamEvent::ReasoningStart {
                index: 0,
                source: None,
            },
            &mut ctx,
        );
        assert_eq!(frames[0].event, "content_block_start");
        assert_eq!(data(&frames[0])["content_block"]["type"], "thinking");

        let frames = stream_event_to_anthropic_sse(
            &StreamEvent::ReasoningDelta("thinking...".into()),
            &mut ctx,
        );
        assert_eq!(data(&frames[0])["delta"]["type"], "thinking_delta");
        assert_eq!(data(&frames[0])["delta"]["thinking"], "thinking...");

        let frames = stream_event_to_anthropic_sse(
            &StreamEvent::ReasoningEnd {
                index: 0,
                signature: "ErWjSig".into(),
            },
            &mut ctx,
        );
        assert_eq!(frames.len(), 2);
        assert_eq!(data(&frames[0])["delta"]["type"], "signature_delta");
        assert_eq!(data(&frames[0])["delta"]["signature"], "ErWjSig");
        assert_eq!(frames[1].event, "content_block_stop");
    }

    #[test]
    fn tool_call_streaming_full_lifecycle() {
        let mut ctx = SseContext::with_model("m");
        feed(
            &mut ctx,
            StreamEvent::ResponseMeta {
                id: "msg_1".into(),
                model: String::new(),
            },
        );
        // Some assistant text first (block index 0).
        feed(&mut ctx, StreamEvent::ContentDelta("ok".into()));

        // Tool call start closes the text block, opens tool block at index 1.
        let frames = stream_event_to_anthropic_sse(
            &StreamEvent::ToolCallStart {
                index: 0,
                id: "toolu_1".into(),
                name: "get_weather".into(),
            },
            &mut ctx,
        );
        assert_eq!(frames[0].event, "content_block_stop");
        assert_eq!(data(&frames[0])["index"], 0);
        assert_eq!(frames[1].event, "content_block_start");
        assert_eq!(data(&frames[1])["index"], 1);
        assert_eq!(data(&frames[1])["content_block"]["type"], "tool_use");
        assert_eq!(data(&frames[1])["content_block"]["id"], "toolu_1");

        let frames = stream_event_to_anthropic_sse(
            &StreamEvent::ToolCallDelta {
                index: 0,
                arguments: r#"{"location":"SF"}"#.into(),
            },
            &mut ctx,
        );
        assert_eq!(data(&frames[0])["delta"]["type"], "input_json_delta");
        assert_eq!(data(&frames[0])["index"], 1);
        assert_eq!(
            data(&frames[0])["delta"]["partial_json"],
            r#"{"location":"SF"}"#
        );

        // Finish closes the tool block; Done emits message_delta + message_stop.
        let frames = stream_event_to_anthropic_sse(
            &StreamEvent::Finish(CanonicalFinishReason::ToolCalls),
            &mut ctx,
        );
        assert_eq!(frames[0].event, "content_block_stop");
        assert_eq!(data(&frames[0])["index"], 1);

        feed(
            &mut ctx,
            StreamEvent::Usage(Usage {
                prompt_tokens: Some(10),
                completion_tokens: Some(7),
                total_tokens: Some(17),
                extra: Default::default(),
            }),
        );
        let frames = stream_event_to_anthropic_sse(&StreamEvent::Done, &mut ctx);
        assert_eq!(frames[0].event, "message_delta");
        assert_eq!(data(&frames[0])["delta"]["stop_reason"], "tool_use");
        assert_eq!(data(&frames[0])["usage"]["output_tokens"], 7);
        assert_eq!(frames[1].event, "message_stop");
    }

    #[test]
    fn full_text_stream_frame_sequence() {
        let mut ctx = SseContext::default();
        let mut events = Vec::new();
        for ev in [
            StreamEvent::ResponseMeta {
                id: "msg_1".into(),
                model: "gpt-4.1".into(),
            },
            StreamEvent::ContentDelta("Hello".into()),
            StreamEvent::Finish(CanonicalFinishReason::Stop),
            StreamEvent::Usage(Usage {
                prompt_tokens: Some(5),
                completion_tokens: Some(1),
                total_tokens: Some(6),
                extra: Default::default(),
            }),
            StreamEvent::Done,
        ] {
            events.extend(stream_event_to_anthropic_sse(&ev, &mut ctx));
        }
        let names: Vec<&str> = events.iter().map(|f| f.event.as_str()).collect();
        assert_eq!(
            names,
            [
                "message_start",
                "content_block_start",
                "content_block_delta",
                "content_block_stop",
                "message_delta",
                "message_stop",
            ]
        );
    }

    #[test]
    fn streaming_usage_carries_input_and_cache_tokens() {
        let mut ctx = SseContext::default();
        feed(
            &mut ctx,
            StreamEvent::ResponseMeta {
                id: "m".into(),
                model: "m".into(),
            },
        );
        feed(&mut ctx, StreamEvent::ContentDelta("hi".into()));
        feed(&mut ctx, StreamEvent::Finish(CanonicalFinishReason::Stop));
        let mut usage = Usage {
            prompt_tokens: Some(120),
            completion_tokens: Some(8),
            total_tokens: Some(128),
            extra: Default::default(),
        };
        // OpenAI surfaces cache hits under prompt_tokens_details.cached_tokens.
        usage.extra.insert(
            "prompt_tokens_details".into(),
            json!({ "cached_tokens": 100 }),
        );
        feed(&mut ctx, StreamEvent::Usage(usage));

        let frames = stream_event_to_anthropic_sse(&StreamEvent::Done, &mut ctx);
        let delta = frames.iter().find(|f| f.event == "message_delta").unwrap();
        let v = data(delta);
        assert_eq!(v["usage"]["input_tokens"], 120);
        assert_eq!(v["usage"]["output_tokens"], 8);
        assert_eq!(v["usage"]["cache_read_input_tokens"], 100);
    }

    #[test]
    fn done_is_idempotent() {
        let mut ctx = SseContext::default();
        feed(
            &mut ctx,
            StreamEvent::ResponseMeta {
                id: "msg_1".into(),
                model: "m".into(),
            },
        );
        let first = stream_event_to_anthropic_sse(&StreamEvent::Done, &mut ctx);
        assert_eq!(first.last().unwrap().event, "message_stop");
        let second = stream_event_to_anthropic_sse(&StreamEvent::Done, &mut ctx);
        assert!(
            second.is_empty(),
            "second Done must not re-emit message_stop"
        );
    }

    #[test]
    fn sse_bytes_render_event_and_data_lines() {
        let frame = AnthropicSseFrame {
            event: "message_stop".to_owned(),
            data: r#"{"type":"message_stop"}"#.to_owned(),
        };
        let bytes = frame.to_sse_bytes();
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n"
        );
    }
}
