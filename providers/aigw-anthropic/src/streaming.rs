//! SSE stream parser for Anthropic's streaming Messages API.
//!
//! Anthropic uses **named SSE events** (`event: message_start`, `event: content_block_delta`, etc.)
//! with JSON `data:` payloads. This module parses the raw byte stream into typed [`StreamEvent`]s.

use bytes::Bytes;
use eventsource_stream::Eventsource;
use futures::stream::{Stream, StreamExt};

use crate::error::Error;
use crate::types::StreamEvent;

/// Parse a raw HTTP byte stream into a stream of typed [`StreamEvent`]s.
///
/// Uses `eventsource-stream` to handle SSE framing, then deserializes
/// each `data:` payload as JSON.
///
/// **Note:** We dispatch on the `type` field inside the JSON `data:` payload,
/// not the SSE `event:` line. Currently Anthropic keeps these two in sync
/// (e.g. `event: content_block_delta` ↔ `{"type": "content_block_delta", …}`),
/// but this is an implementation detail of Anthropic, not an SSE-level guarantee.
///
/// # Errors
///
/// - [`Error::Json`] — the `data:` payload is non-empty but fails to deserialize
///   as a [`StreamEvent`] (e.g. malformed JSON or unexpected schema).
/// - [`Error::Stream`] — the underlying byte stream or SSE framing layer
///   produced a transport-level error.
pub fn parse_sse_stream<S>(byte_stream: S) -> impl Stream<Item = Result<StreamEvent, Error>> + Send
where
    S: Stream<Item = Result<Bytes, std::io::Error>> + Send + Unpin + 'static,
{
    byte_stream.eventsource().filter_map(|result| async move {
        match result {
            Ok(event) => {
                let data = event.data.trim();

                if data.is_empty() {
                    return None;
                }

                match serde_json::from_str::<StreamEvent>(data) {
                    Ok(stream_event) => Some(Ok(stream_event)),
                    Err(e) => Some(Err(Error::Json(e))),
                }
            }
            Err(e) => Some(Err(Error::stream(e))),
        }
    })
}
