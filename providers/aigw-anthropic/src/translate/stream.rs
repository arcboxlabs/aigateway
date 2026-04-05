//! Stream translation: Anthropic SSE events → canonical `StreamEvent`s.
//!
//! Anthropic uses named SSE events with block-level granularity. The parser
//! maintains state across events to track tool call indices and combine
//! input_tokens (from message_start) with output_tokens (from message_delta).

use aigw_core::error::TranslateError;
use aigw_core::model::{StreamEvent as CanonicalStreamEvent, Usage};
use aigw_core::translate::StreamParser;

use crate::types::{
    ContentBlock, ContentDelta, StreamEvent as AnthropicStreamEvent, TypedContentBlock,
};

/// Stateful parser for Anthropic SSE streams.
///
/// Created per-request via [`AnthropicResponseTranslator::stream_parser()`].
pub struct AnthropicStreamParser {
    /// Incremented on each `content_block_start` with `tool_use` type.
    tool_call_index: u32,
    /// Input token count from `message_start`.
    input_tokens: Option<u64>,
    /// Whether `Done` has been emitted.
    done: bool,
}

impl Default for AnthropicStreamParser {
    fn default() -> Self {
        Self::new()
    }
}

impl AnthropicStreamParser {
    pub fn new() -> Self {
        Self {
            tool_call_index: 0,
            input_tokens: None,
            done: false,
        }
    }
}

impl StreamParser for AnthropicStreamParser {
    fn parse_event(
        &mut self,
        _event_type: &str,
        data: &str,
    ) -> Result<Vec<CanonicalStreamEvent>, TranslateError> {
        let native: AnthropicStreamEvent =
            serde_json::from_str(data).map_err(|e| TranslateError::StreamParse {
                message: format!("failed to parse Anthropic stream event: {e}"),
            })?;

        match native {
            AnthropicStreamEvent::MessageStart { message } => {
                self.input_tokens = Some(message.usage.input_tokens);
                Ok(vec![CanonicalStreamEvent::ResponseMeta {
                    id: message.id,
                    model: message.model,
                }])
            }

            AnthropicStreamEvent::ContentBlockStart { content_block, .. } => {
                match &content_block {
                    ContentBlock::Typed(TypedContentBlock::ToolUse { id, name, .. }) => {
                        let idx = self.tool_call_index;
                        self.tool_call_index += 1;
                        Ok(vec![CanonicalStreamEvent::ToolCallStart {
                            index: idx,
                            id: id.clone(),
                            name: name.clone(),
                        }])
                    }
                    // Text block start, Thinking, etc: no output.
                    _ => Ok(vec![]),
                }
            }

            AnthropicStreamEvent::ContentBlockDelta { delta, .. } => match delta {
                ContentDelta::TextDelta { text } => {
                    Ok(vec![CanonicalStreamEvent::ContentDelta(text)])
                }
                ContentDelta::InputJsonDelta { partial_json } => {
                    let tool_idx = self.tool_call_index.saturating_sub(1);
                    Ok(vec![CanonicalStreamEvent::ToolCallDelta {
                        index: tool_idx,
                        arguments: partial_json,
                    }])
                }
                // ThinkingDelta, SignatureDelta, Unknown: skip.
                _ => Ok(vec![]),
            },

            AnthropicStreamEvent::ContentBlockStop { .. } => Ok(vec![]),

            AnthropicStreamEvent::MessageDelta { delta, usage } => {
                let mut events = Vec::new();

                if let Some(stop_reason) = delta.stop_reason {
                    events.push(CanonicalStreamEvent::Finish(stop_reason.into()));
                }

                let output_tokens = usage.output_tokens;
                let input_tokens = self.input_tokens.unwrap_or(0);
                events.push(CanonicalStreamEvent::Usage(Usage {
                    prompt_tokens: Some(input_tokens),
                    completion_tokens: Some(output_tokens),
                    total_tokens: Some(input_tokens + output_tokens),
                    extra: Default::default(),
                }));

                Ok(events)
            }

            AnthropicStreamEvent::MessageStop => {
                self.done = true;
                Ok(vec![CanonicalStreamEvent::Done])
            }

            AnthropicStreamEvent::Ping => Ok(vec![]),

            AnthropicStreamEvent::Error { error } => Err(TranslateError::StreamParse {
                message: format!(
                    "Anthropic stream error: [{}] {}",
                    error.r#type, error.message
                ),
            }),

            AnthropicStreamEvent::Unknown => Ok(vec![]),
        }
    }

    fn finish(&mut self) -> Result<Vec<CanonicalStreamEvent>, TranslateError> {
        if !self.done {
            self.done = true;
            Ok(vec![CanonicalStreamEvent::Done])
        } else {
            Ok(vec![])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aigw_core::model::FinishReason;

    fn parser() -> AnthropicStreamParser {
        AnthropicStreamParser::new()
    }

    #[test]
    fn message_start_emits_response_meta() {
        let mut p = parser();
        let data = r#"{
            "type": "message_start",
            "message": {
                "id": "msg_01",
                "type": "message",
                "role": "assistant",
                "content": [],
                "model": "claude-sonnet-4-20250514",
                "stop_reason": null,
                "stop_sequence": null,
                "usage": { "input_tokens": 25, "output_tokens": 0 }
            }
        }"#;

        let events = p.parse_event("message_start", data).unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            CanonicalStreamEvent::ResponseMeta { id, model }
            if id == "msg_01" && model == "claude-sonnet-4-20250514"
        ));
        assert_eq!(p.input_tokens, Some(25));
    }

    #[test]
    fn text_content_delta() {
        let mut p = parser();

        // content_block_start (text) → no output
        let start = r#"{"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}}"#;
        let events = p.parse_event("content_block_start", start).unwrap();
        assert!(events.is_empty());

        // content_block_delta (text_delta) → ContentDelta
        let delta = r#"{"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Hello"}}"#;
        let events = p.parse_event("content_block_delta", delta).unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            CanonicalStreamEvent::ContentDelta(s) if s == "Hello"
        ));
    }

    #[test]
    fn tool_call_streaming() {
        let mut p = parser();

        // content_block_start (tool_use) → ToolCallStart
        let start = r#"{
            "type": "content_block_start",
            "index": 1,
            "content_block": {
                "type": "tool_use",
                "id": "toolu_01",
                "name": "get_weather",
                "input": {}
            }
        }"#;
        let events = p.parse_event("content_block_start", start).unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            CanonicalStreamEvent::ToolCallStart { index: 0, id, name }
            if id == "toolu_01" && name == "get_weather"
        ));

        // content_block_delta (input_json_delta) → ToolCallDelta
        let delta = r#"{"type": "content_block_delta", "index": 1, "delta": {"type": "input_json_delta", "partial_json": "{\"loc"}}"#;
        let events = p.parse_event("content_block_delta", delta).unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            CanonicalStreamEvent::ToolCallDelta { index: 0, arguments }
            if arguments == "{\"loc"
        ));
    }

    #[test]
    fn multiple_tool_calls_increment_index() {
        let mut p = parser();

        // First tool
        let start1 = r#"{"type": "content_block_start", "index": 0, "content_block": {"type": "tool_use", "id": "t1", "name": "fn1", "input": {}}}"#;
        let events = p.parse_event("", start1).unwrap();
        assert!(matches!(
            &events[0],
            CanonicalStreamEvent::ToolCallStart { index: 0, .. }
        ));

        // Second tool
        let start2 = r#"{"type": "content_block_start", "index": 1, "content_block": {"type": "tool_use", "id": "t2", "name": "fn2", "input": {}}}"#;
        let events = p.parse_event("", start2).unwrap();
        assert!(matches!(
            &events[0],
            CanonicalStreamEvent::ToolCallStart { index: 1, .. }
        ));
    }

    #[test]
    fn message_delta_emits_finish_and_usage() {
        let mut p = parser();
        p.input_tokens = Some(25);

        let data = r#"{
            "type": "message_delta",
            "delta": { "stop_reason": "end_turn", "stop_sequence": null },
            "usage": { "output_tokens": 15 }
        }"#;

        let events = p.parse_event("message_delta", data).unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(
            &events[0],
            CanonicalStreamEvent::Finish(FinishReason::Stop)
        ));
        match &events[1] {
            CanonicalStreamEvent::Usage(u) => {
                assert_eq!(u.prompt_tokens, Some(25));
                assert_eq!(u.completion_tokens, Some(15));
                assert_eq!(u.total_tokens, Some(40));
            }
            other => panic!("expected Usage, got {other:?}"),
        }
    }

    #[test]
    fn message_stop_emits_done() {
        let mut p = parser();
        let data = r#"{"type": "message_stop"}"#;
        let events = p.parse_event("message_stop", data).unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], CanonicalStreamEvent::Done));
        assert!(p.done);
    }

    #[test]
    fn ping_is_ignored() {
        let mut p = parser();
        let data = r#"{"type": "ping"}"#;
        let events = p.parse_event("ping", data).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn error_event_returns_err() {
        let mut p = parser();
        let data =
            r#"{"type": "error", "error": {"type": "overloaded_error", "message": "Overloaded"}}"#;
        let err = p.parse_event("error", data).unwrap_err();
        assert!(matches!(err, TranslateError::StreamParse { .. }));
    }

    #[test]
    fn finish_emits_done_if_not_already() {
        let mut p = parser();
        let events = p.finish().unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], CanonicalStreamEvent::Done));

        // Second call: no duplicate.
        let events = p.finish().unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn full_stream_replay() {
        let mut p = parser();
        let mut all_events = Vec::new();

        let sequence = [
            r#"{"type":"message_start","message":{"id":"msg_01","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-20250514","stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":25,"output_tokens":0}}}"#,
            r#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#,
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":" world"}}"#,
            r#"{"type":"content_block_stop","index":0}"#,
            r#"{"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":5}}"#,
            r#"{"type":"message_stop"}"#,
        ];

        for data in sequence {
            all_events.extend(p.parse_event("", data).unwrap());
        }

        // Verify the sequence.
        assert!(matches!(
            &all_events[0],
            CanonicalStreamEvent::ResponseMeta { .. }
        ));
        assert!(matches!(&all_events[1], CanonicalStreamEvent::ContentDelta(s) if s == "Hello"));
        assert!(matches!(&all_events[2], CanonicalStreamEvent::ContentDelta(s) if s == " world"));
        assert!(matches!(
            &all_events[3],
            CanonicalStreamEvent::Finish(FinishReason::Stop)
        ));
        assert!(matches!(&all_events[4], CanonicalStreamEvent::Usage(_)));
        assert!(matches!(&all_events[5], CanonicalStreamEvent::Done));
    }
}
