//! Response translation: OpenAI Responses API → canonical `ChatResponse`.
//!
//! Converts `ResponseObject` into the canonical Chat Completions response
//! format: output items become a single `Choice` with message content and
//! tool calls. Reasoning output items are mapped to `reasoning_content` in
//! the message `extra` field.

use aigw_core::error::{ProviderError, TranslateError, map_error_status};
use aigw_core::model::{
    ChatResponse, Choice, FinishReason, FunctionCall, Message, MessageContent, Role, ToolCall,
    Usage,
};
use aigw_core::translate::{ResponseTranslator, StreamParser};
use http::{HeaderMap, StatusCode};

use crate::wire_types::{
    ApiErrorResponse, ResponseContentPart, ResponseObject, ResponseOutputItem,
    ResponseReasoningSummaryPart,
};

use super::responses_stream::ResponsesStreamParser;

/// Translates OpenAI Responses API responses into canonical types.
pub struct ResponsesResponseTranslator;

impl ResponseTranslator for ResponsesResponseTranslator {
    fn translate_response(
        &self,
        _status: StatusCode,
        body: &[u8],
    ) -> Result<ChatResponse, TranslateError> {
        let resp: ResponseObject = serde_json::from_slice(body)?;
        Ok(response_object_to_chat_response(resp))
    }

    fn stream_parser(&self) -> Box<dyn StreamParser> {
        Box::new(ResponsesStreamParser::new())
    }

    fn translate_error(
        &self,
        status: StatusCode,
        headers: &HeaderMap,
        body: &[u8],
    ) -> ProviderError {
        let parsed = serde_json::from_slice::<ApiErrorResponse>(body);
        let message = parsed
            .as_ref()
            .map(|e| e.error.message.clone())
            .unwrap_or_else(|_| String::from_utf8_lossy(body).into_owned());

        map_error_status(status.as_u16(), headers, message)
    }
}

/// Convert a `ResponseObject` into a canonical `ChatResponse`.
fn response_object_to_chat_response(resp: ResponseObject) -> ChatResponse {
    let mut text_parts: Vec<String> = Vec::new();
    let mut tool_calls: Vec<ToolCall> = Vec::new();
    let mut reasoning_text: Option<String> = None;

    if let Some(output) = &resp.output {
        for item in output {
            match item {
                ResponseOutputItem::Message { content, .. } => {
                    for part in content {
                        if let ResponseContentPart::OutputText { text, .. } = part {
                            text_parts.push(text.clone());
                        }
                    }
                }
                ResponseOutputItem::FunctionCall {
                    arguments,
                    call_id,
                    name,
                    id,
                    ..
                } => {
                    tool_calls.push(ToolCall {
                        id: id.clone().unwrap_or_else(|| call_id.clone()),
                        kind: "function".into(),
                        function: FunctionCall {
                            name: name.clone(),
                            arguments: arguments.clone(),
                            extra: Default::default(),
                        },
                        extra: Default::default(),
                    });
                }
                ResponseOutputItem::Reasoning { summary, .. } => {
                    let texts: Vec<&str> = summary
                        .iter()
                        .filter_map(|p| match p {
                            ResponseReasoningSummaryPart::SummaryText { text, .. } => {
                                Some(text.as_str())
                            }
                            _ => None,
                        })
                        .collect();
                    if !texts.is_empty() {
                        reasoning_text = Some(texts.join(""));
                    }
                }
                _ => {}
            }
        }
    }

    let content = if text_parts.is_empty() {
        if tool_calls.is_empty() {
            None
        } else {
            // Content must be null (not absent) when tool_calls are present,
            // matching OpenAI Chat Completions behaviour.
            None
        }
    } else {
        Some(MessageContent::Text(text_parts.join("")))
    };

    let tool_calls_field = if tool_calls.is_empty() {
        None
    } else {
        Some(tool_calls)
    };

    let finish_reason = match resp.status.as_deref() {
        Some("completed") => {
            if tool_calls_field.is_some() {
                Some(FinishReason::ToolCalls)
            } else {
                Some(FinishReason::Stop)
            }
        }
        Some("incomplete") => Some(FinishReason::Length),
        Some("cancelled") => Some(FinishReason::Stop),
        Some("failed") => Some(FinishReason::Stop),
        Some(other) => Some(FinishReason::Unknown(other.to_owned())),
        None => None,
    };

    let usage = resp.usage.map(|u| {
        let mut extra = serde_json::Map::new();
        // Map input_tokens_details → prompt_tokens_details.
        if let Some(details) = u.input_tokens_details.clone() {
            extra.insert("prompt_tokens_details".into(), details);
        }
        // Map output_tokens_details → completion_tokens_details.
        if let Some(details) = u.output_tokens_details.clone() {
            extra.insert("completion_tokens_details".into(), details);
        }
        Usage {
            prompt_tokens: u.input_tokens,
            completion_tokens: u.output_tokens,
            total_tokens: u.total_tokens,
            extra,
        }
    });

    let model = resp.model.unwrap_or_default();
    let created = resp.created_at.unwrap_or(0);

    // Reasoning content flows through the message's `extra` field so it
    // round-trips back to clients that understand it.
    let mut msg_extra = serde_json::Map::new();
    if let Some(r) = reasoning_text {
        msg_extra.insert("reasoning_content".into(), serde_json::Value::String(r));
    }

    ChatResponse {
        id: resp.id,
        object: "chat.completion".into(),
        created,
        model,
        choices: vec![Choice {
            index: 0,
            message: Message {
                role: Role::Assistant,
                content,
                name: None,
                tool_call_id: None,
                tool_calls: tool_calls_field,
                extra: msg_extra,
            },
            finish_reason,
            extra: Default::default(),
        }],
        usage,
        extra: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use aigw_core::model::FinishReason;
    use aigw_core::translate::ResponseTranslator;
    use http::StatusCode;

    use super::ResponsesResponseTranslator;

    #[test]
    fn translate_message_output() {
        let json = r#"{
            "id": "resp_123",
            "object": "response",
            "created_at": 1700000000,
            "status": "completed",
            "model": "gpt-4.1",
            "output": [{
                "type": "message",
                "id": "msg_1",
                "role": "assistant",
                "status": "completed",
                "content": [{ "type": "output_text", "text": "Hello!" }]
            }],
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5,
                "total_tokens": 15
            }
        }"#;

        let translator = ResponsesResponseTranslator;
        let resp = translator
            .translate_response(StatusCode::OK, json.as_bytes())
            .unwrap();

        assert_eq!(resp.id, "resp_123");
        assert_eq!(resp.model, "gpt-4.1");
        assert_eq!(resp.choices.len(), 1);
        assert_eq!(resp.choices[0].finish_reason, Some(FinishReason::Stop));
        let usage = resp.usage.as_ref().unwrap();
        assert_eq!(usage.prompt_tokens, Some(10));
        assert_eq!(usage.completion_tokens, Some(5));
    }

    #[test]
    fn translate_function_call_output() {
        let json = r#"{
            "id": "resp_456",
            "object": "response",
            "created_at": 1700000000,
            "status": "completed",
            "model": "gpt-4.1",
            "output": [{
                "type": "function_call",
                "id": "fc_1",
                "call_id": "call_abc",
                "name": "get_weather",
                "arguments": "{\"location\":\"SF\"}",
                "status": "completed"
            }]
        }"#;

        let translator = ResponsesResponseTranslator;
        let resp = translator
            .translate_response(StatusCode::OK, json.as_bytes())
            .unwrap();

        assert_eq!(resp.choices[0].finish_reason, Some(FinishReason::ToolCalls));
        let tc = resp.choices[0].message.tool_calls.as_ref().unwrap();
        assert_eq!(tc[0].function.name, "get_weather");
        assert_eq!(tc[0].id, "fc_1");
    }

    #[test]
    fn translate_incomplete_status() {
        let json = r#"{
            "id": "resp_789",
            "object": "response",
            "status": "incomplete",
            "model": "gpt-4.1",
            "output": [{
                "type": "message",
                "id": "msg_1",
                "role": "assistant",
                "content": [{ "type": "output_text", "text": "partial..." }]
            }]
        }"#;

        let translator = ResponsesResponseTranslator;
        let resp = translator
            .translate_response(StatusCode::OK, json.as_bytes())
            .unwrap();

        assert_eq!(resp.choices[0].finish_reason, Some(FinishReason::Length));
    }

    #[test]
    fn translate_usage_details() {
        let json = r#"{
            "id": "resp_u",
            "object": "response",
            "status": "completed",
            "model": "o4-mini",
            "output": [{
                "type": "message",
                "id": "msg_1",
                "role": "assistant",
                "content": [{ "type": "output_text", "text": "ok" }]
            }],
            "usage": {
                "input_tokens": 100,
                "output_tokens": 50,
                "total_tokens": 150,
                "input_tokens_details": { "cached_tokens": 80 },
                "output_tokens_details": { "reasoning_tokens": 30 }
            }
        }"#;

        let translator = ResponsesResponseTranslator;
        let resp = translator
            .translate_response(StatusCode::OK, json.as_bytes())
            .unwrap();

        let usage = resp.usage.unwrap();
        assert_eq!(usage.prompt_tokens, Some(100));
        assert_eq!(usage.completion_tokens, Some(50));
        assert_eq!(
            usage.extra.get("prompt_tokens_details").unwrap()["cached_tokens"],
            80
        );
        assert_eq!(
            usage.extra.get("completion_tokens_details").unwrap()["reasoning_tokens"],
            30
        );
    }

    #[test]
    fn translate_reasoning_output() {
        let json = r#"{
            "id": "resp_r",
            "object": "response",
            "created_at": 1700000000,
            "status": "completed",
            "model": "o4-mini",
            "output": [
                {
                    "type": "reasoning",
                    "id": "rs_1",
                    "summary": [{ "type": "summary_text", "text": "Thinking about it..." }]
                },
                {
                    "type": "message",
                    "id": "msg_1",
                    "role": "assistant",
                    "content": [{ "type": "output_text", "text": "Done." }]
                }
            ],
            "usage": { "input_tokens": 10, "output_tokens": 20, "total_tokens": 30 }
        }"#;

        let translator = ResponsesResponseTranslator;
        let resp = translator
            .translate_response(StatusCode::OK, json.as_bytes())
            .unwrap();

        assert_eq!(
            resp.choices[0]
                .message
                .extra
                .get("reasoning_content")
                .unwrap(),
            "Thinking about it..."
        );
    }
}
