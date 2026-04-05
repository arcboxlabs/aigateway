//! Translation layer: canonical ↔ Anthropic Messages API.
//!
//! Unlike the OpenAI translator (near-passthrough), Anthropic translation
//! requires significant restructuring:
//! - System messages are extracted to a top-level field
//! - Tool definitions are unwrapped from the OpenAI `function` wrapper
//! - Tool results are restructured into user messages with content blocks
//! - Streaming events use a different granularity (block-level vs choice-level)

pub mod request;
pub mod response;
pub mod stream;
pub mod tools;

pub use request::AnthropicRequestTranslator;
pub use response::AnthropicResponseTranslator;
pub use stream::AnthropicStreamParser;

use crate::types::StopReason;
use aigw_core::model::FinishReason;

/// Canonical conversion: Anthropic stop reason → canonical finish reason.
///
/// Used by both `response.rs` and `stream.rs` to avoid duplication.
impl From<StopReason> for FinishReason {
    fn from(reason: StopReason) -> Self {
        match reason {
            StopReason::EndTurn | StopReason::StopSequence => FinishReason::Stop,
            StopReason::MaxTokens => FinishReason::Length,
            StopReason::ToolUse => FinishReason::ToolCalls,
            StopReason::Other(s) => FinishReason::Unknown(s),
        }
    }
}
