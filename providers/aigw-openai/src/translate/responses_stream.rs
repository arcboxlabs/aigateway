//! Stream translation: OpenAI Responses API SSE events → canonical `StreamEvent`s.
//!
//! Responses API events use a typed `type` field (e.g. `response.output_text.delta`,
//! `response.completed`) instead of the Chat Completions `chat.completion.chunk`
//! format. This parser maintains state across events to track tool call indices
//! and reasoning blocks (encrypted_content / reasoning_signature).

use aigw_core::error::TranslateError;
use aigw_core::model::{FinishReason, StreamEvent, Usage};
use aigw_core::translate::StreamParser;

/// Parses OpenAI Responses API SSE events into canonical streaming events.
pub struct ResponsesStreamParser {
    meta_emitted: bool,
    tool_call_index: u32,
    /// Buffered `encrypted_content` from a reasoning `output_item.added` event.
    /// Emitted as `ReasoningSignature` when the reasoning block finalizes.
    thinking_signature: Option<String>,
    /// True while a reasoning output item is open (between added and done).
    thinking_block_open: bool,
}

impl Default for ResponsesStreamParser {
    fn default() -> Self {
        Self::new()
    }
}

impl ResponsesStreamParser {
    pub fn new() -> Self {
        Self {
            meta_emitted: false,
            tool_call_index: 0,
            thinking_signature: None,
            thinking_block_open: false,
        }
    }

    /// Flush a pending reasoning signature, returning an event if one was buffered.
    fn flush_signature(&mut self) -> Option<StreamEvent> {
        self.thinking_block_open = false;
        self.thinking_signature
            .take()
            .map(StreamEvent::ReasoningSignature)
    }
}

impl StreamParser for ResponsesStreamParser {
    fn parse_event(
        &mut self,
        _event_type: &str,
        data: &str,
    ) -> Result<Vec<StreamEvent>, TranslateError> {
        if data.trim() == "[DONE]" {
            return Ok(vec![StreamEvent::Done]);
        }

        let ev: serde_json::Value =
            serde_json::from_str(data).map_err(|e| TranslateError::StreamParse {
                message: format!("failed to parse Responses SSE event: {e}"),
            })?;

        let event_type = ev["type"].as_str().unwrap_or("");
        let mut events = Vec::new();

        match event_type {
            "response.created" => {
                if !self.meta_emitted {
                    let id = ev
                        .pointer("/response/id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_owned();
                    let model = ev
                        .pointer("/response/model")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_owned();
                    events.push(StreamEvent::ResponseMeta { id, model });
                    self.meta_emitted = true;
                }
            }

            // ── Reasoning events ────────────────────────────────────────

            "response.output_item.added"
                if ev.pointer("/item/type").and_then(|v| v.as_str()) == Some("reasoning") =>
            {
                self.thinking_block_open = true;
                self.thinking_signature = ev
                    .pointer("/item/encrypted_content")
                    .and_then(|v| v.as_str())
                    .map(String::from);
            }

            "response.output_item.done"
                if ev.pointer("/item/type").and_then(|v| v.as_str()) == Some("reasoning") =>
            {
                if let Some(ev) = self.flush_signature() {
                    events.push(ev);
                }
            }

            "response.reasoning_summary_text.delta" => {
                let delta = ev["delta"].as_str().unwrap_or("").to_owned();
                if !delta.is_empty() {
                    events.push(StreamEvent::ReasoningDelta(delta));
                }
            }

            "response.reasoning_summary_text.done" => {
                // Insert a separator between multi-segment reasoning summaries,
                // matching CLIProxy's behaviour.
                events.push(StreamEvent::ReasoningDelta("\n\n".into()));
            }

            // ── Content events ──────────────────────────────────────────

            "response.output_text.delta" => {
                let delta = ev["delta"].as_str().unwrap_or("").to_owned();
                if !delta.is_empty() {
                    events.push(StreamEvent::ContentDelta(delta));
                }
            }

            // ── Tool call events ────────────────────────────────────────

            "response.output_item.added"
                if ev.pointer("/item/type").and_then(|v| v.as_str())
                    == Some("function_call") =>
            {
                // Flush pending reasoning signature before tool calls start.
                if let Some(sig) = self.flush_signature() {
                    events.push(sig);
                }

                let id = ev
                    .pointer("/item/call_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned();
                let name = ev
                    .pointer("/item/name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned();
                let index = self.tool_call_index;
                self.tool_call_index += 1;
                events.push(StreamEvent::ToolCallStart { index, id, name });
            }

            "response.function_call_arguments.delta" => {
                let delta = ev["delta"].as_str().unwrap_or("").to_owned();
                if !delta.is_empty() {
                    let index = self.tool_call_index.saturating_sub(1);
                    events.push(StreamEvent::ToolCallDelta {
                        index,
                        arguments: delta,
                    });
                }
            }

            // ── Completion ──────────────────────────────────────────────

            "response.completed" => {
                let status = ev
                    .pointer("/response/status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("completed");

                let reason = if self.tool_call_index > 0 {
                    FinishReason::ToolCalls
                } else {
                    match status {
                        "completed" => FinishReason::Stop,
                        "incomplete" => FinishReason::Length,
                        other => FinishReason::Unknown(other.to_owned()),
                    }
                };
                events.push(StreamEvent::Finish(reason));

                let input_tokens = ev
                    .pointer("/response/usage/input_tokens")
                    .and_then(|v| v.as_u64());
                let output_tokens = ev
                    .pointer("/response/usage/output_tokens")
                    .and_then(|v| v.as_u64());
                let total_tokens = ev
                    .pointer("/response/usage/total_tokens")
                    .and_then(|v| v.as_u64());

                let mut usage_extra = serde_json::Map::new();
                if let Some(details) = ev.pointer("/response/usage/input_tokens_details") {
                    usage_extra.insert("prompt_tokens_details".into(), details.clone());
                }
                if let Some(details) = ev.pointer("/response/usage/output_tokens_details") {
                    usage_extra.insert("completion_tokens_details".into(), details.clone());
                }

                events.push(StreamEvent::Usage(Usage {
                    prompt_tokens: input_tokens,
                    completion_tokens: output_tokens,
                    total_tokens,
                    extra: usage_extra,
                }));

                events.push(StreamEvent::Done);
            }

            _ => {}
        }

        Ok(events)
    }

    fn finish(&mut self) -> Result<Vec<StreamEvent>, TranslateError> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use aigw_core::model::{FinishReason, StreamEvent};
    use aigw_core::translate::StreamParser;

    use super::ResponsesStreamParser;

    fn parser() -> ResponsesStreamParser {
        ResponsesStreamParser::new()
    }

    #[test]
    fn response_created_emits_meta() {
        let mut p = parser();
        let data = r#"{
            "type": "response.created",
            "response": { "id": "resp_abc", "object": "response", "model": "gpt-4.1", "created_at": 1 }
        }"#;

        let events = p.parse_event("", data).unwrap();
        assert!(matches!(
            &events[0],
            StreamEvent::ResponseMeta { id, model }
            if id == "resp_abc" && model == "gpt-4.1"
        ));
    }

    #[test]
    fn text_delta() {
        let mut p = parser();
        let data = r#"{"type": "response.output_text.delta", "delta": "Hello"}"#;
        let events = p.parse_event("", data).unwrap();
        assert!(matches!(&events[0], StreamEvent::ContentDelta(s) if s == "Hello"));
    }

    #[test]
    fn reasoning_summary_delta() {
        let mut p = parser();
        let data = r#"{"type": "response.reasoning_summary_text.delta", "delta": "Thinking..."}"#;
        let events = p.parse_event("", data).unwrap();
        assert!(matches!(&events[0], StreamEvent::ReasoningDelta(s) if s == "Thinking..."));
    }

    #[test]
    fn reasoning_signature_emitted_on_block_done() {
        let mut p = parser();

        let added = r#"{
            "type": "response.output_item.added",
            "item": { "type": "reasoning", "id": "rs_1", "encrypted_content": "sig_abc123" }
        }"#;
        let events = p.parse_event("", added).unwrap();
        assert!(events.is_empty(), "added should buffer, not emit");

        let done = r#"{
            "type": "response.output_item.done",
            "item": { "type": "reasoning", "id": "rs_1" }
        }"#;
        let events = p.parse_event("", done).unwrap();
        assert!(matches!(&events[0], StreamEvent::ReasoningSignature(s) if s == "sig_abc123"));
    }

    #[test]
    fn reasoning_signature_flushed_before_tool_call() {
        let mut p = parser();

        let added = r#"{
            "type": "response.output_item.added",
            "item": { "type": "reasoning", "id": "rs_1", "encrypted_content": "sig_xyz" }
        }"#;
        p.parse_event("", added).unwrap();

        // Tool call starts before reasoning block is explicitly "done".
        let tc = r#"{
            "type": "response.output_item.added",
            "item": { "type": "function_call", "call_id": "c1", "name": "f" }
        }"#;
        let events = p.parse_event("", tc).unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(&events[0], StreamEvent::ReasoningSignature(s) if s == "sig_xyz"));
        assert!(matches!(&events[1], StreamEvent::ToolCallStart { index: 0, .. }));
    }

    #[test]
    fn function_call_start_and_delta() {
        let mut p = parser();

        let start = r#"{
            "type": "response.output_item.added",
            "item": { "type": "function_call", "call_id": "call_1", "name": "search" }
        }"#;
        let events = p.parse_event("", start).unwrap();
        assert!(matches!(
            &events[0],
            StreamEvent::ToolCallStart { index: 0, id, name }
            if id == "call_1" && name == "search"
        ));

        let delta = r#"{"type": "response.function_call_arguments.delta", "delta": "{\"q\":"}"#;
        let events = p.parse_event("", delta).unwrap();
        assert!(matches!(
            &events[0],
            StreamEvent::ToolCallDelta { index: 0, arguments }
            if arguments == "{\"q\":"
        ));
    }

    #[test]
    fn response_completed_emits_finish_usage_done() {
        let mut p = parser();
        let data = r#"{
            "type": "response.completed",
            "response": {
                "id": "resp_abc",
                "status": "completed",
                "usage": { "input_tokens": 10, "output_tokens": 5, "total_tokens": 15 }
            }
        }"#;

        let events = p.parse_event("", data).unwrap();
        assert_eq!(events.len(), 3);
        assert!(matches!(&events[0], StreamEvent::Finish(FinishReason::Stop)));
        assert!(matches!(&events[1], StreamEvent::Usage(u) if u.prompt_tokens == Some(10)));
        assert!(matches!(&events[2], StreamEvent::Done));
    }

    #[test]
    fn tool_calls_set_finish_reason() {
        let mut p = parser();

        let start = r#"{
            "type": "response.output_item.added",
            "item": { "type": "function_call", "call_id": "c1", "name": "f" }
        }"#;
        p.parse_event("", start).unwrap();

        let completed = r#"{
            "type": "response.completed",
            "response": { "status": "completed", "usage": { "input_tokens": 1, "output_tokens": 1 } }
        }"#;
        let events = p.parse_event("", completed).unwrap();
        assert!(matches!(
            &events[0],
            StreamEvent::Finish(FinishReason::ToolCalls)
        ));
    }

    #[test]
    fn reasoning_summary_text_done_emits_separator() {
        let mut p = parser();
        let data = r#"{"type": "response.reasoning_summary_text.done"}"#;
        let events = p.parse_event("", data).unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], StreamEvent::ReasoningDelta(s) if s == "\n\n"));
    }

    #[test]
    fn response_completed_maps_usage_details() {
        let mut p = parser();
        let data = r#"{
            "type": "response.completed",
            "response": {
                "status": "completed",
                "usage": {
                    "input_tokens": 100,
                    "output_tokens": 50,
                    "total_tokens": 150,
                    "input_tokens_details": { "cached_tokens": 80 },
                    "output_tokens_details": { "reasoning_tokens": 30 }
                }
            }
        }"#;
        let events = p.parse_event("", data).unwrap();
        assert_eq!(events.len(), 3);
        match &events[1] {
            StreamEvent::Usage(u) => {
                assert_eq!(u.prompt_tokens, Some(100));
                assert_eq!(
                    u.extra.get("prompt_tokens_details").unwrap()["cached_tokens"],
                    80
                );
                assert_eq!(
                    u.extra.get("completion_tokens_details").unwrap()["reasoning_tokens"],
                    30
                );
            }
            other => panic!("expected Usage, got {other:?}"),
        }
    }

    #[test]
    fn done_event() {
        let mut p = parser();
        let events = p.parse_event("", "[DONE]").unwrap();
        assert!(matches!(&events[0], StreamEvent::Done));
    }

    #[test]
    fn finish_returns_empty() {
        let mut p = parser();
        assert!(p.finish().unwrap().is_empty());
    }
}
