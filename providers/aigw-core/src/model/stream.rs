//! Canonical streaming event types.
//!
//! These are the intermediate events produced by [`StreamParser`](crate::translate::StreamParser)
//! implementations. The gateway assembles them into OpenAI-format `ChatCompletionChunk`
//! objects for the client.
//!
//! The design is deliberately more granular than any single provider's event model.
//! For example, Anthropic's `content_block_start(tool_use)` + `content_block_delta(input_json)`
//! maps to `ToolCallStart` + `ToolCallDelta`, while OpenAI's single delta chunk with
//! `tool_calls[].function.name` and `tool_calls[].function.arguments` maps to the same
//! pair.

use super::response::{FinishReason, Usage};

/// A canonical streaming event.
///
/// Stream parsers produce a sequence of these events. The gateway consumes
/// them to build OpenAI-format `ChatCompletionChunk` responses.
///
/// Typical event order:
/// ```text
/// ResponseMeta → ContentDelta* → (ToolCallStart → ToolCallDelta*)* → Finish → Usage → Done
/// ```
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// First event — establishes the response identity.
    /// Captured from OpenAI's first chunk, Anthropic's `message_start`,
    /// or generated for Gemini.
    ResponseMeta {
        /// Response ID (e.g. `"chatcmpl-xxx"`, `"msg_xxx"`).
        id: String,
        /// Model identifier.
        model: String,
    },

    /// Incremental text content.
    ContentDelta(String),

    /// A new tool call begins.
    ///
    /// `index` is the zero-based position in the `tool_calls` array.
    /// For Gemini, the `id` is a generated UUID since Gemini doesn't provide one.
    ToolCallStart {
        index: u32,
        id: String,
        name: String,
    },

    /// Incremental tool call arguments (partial JSON string).
    ToolCallDelta {
        index: u32,
        arguments: String,
    },

    /// The model has finished generating.
    Finish(FinishReason),

    /// Token usage statistics (typically arrives at/near end of stream).
    Usage(Usage),

    /// Stream is complete — no more events will follow.
    Done,
}
