//! SSE stream parser for Gemini's streaming generateContent API.
//!
//! Gemini streaming uses standard SSE with `data:` lines only (no named `event:` types).
//! Each `data:` payload is a complete [`GenerateContentResponse`] JSON object.
//! The stream ends when the HTTP connection closes — there is no `[DONE]` sentinel.
//!
//! Enable streaming by appending `?alt=sse` to the `streamGenerateContent` endpoint.

use bytes::Bytes;
use eventsource_stream::Eventsource;
use futures::stream::{Stream, StreamExt};

use crate::error::Error;
use crate::types::GenerateContentResponse;

/// Parse a raw HTTP byte stream into a stream of typed [`GenerateContentResponse`]s.
///
/// Uses `eventsource-stream` to handle SSE framing, then deserializes
/// each `data:` payload as JSON.
///
/// Unlike Anthropic (which uses named SSE events and delta-based streaming),
/// Gemini sends a complete [`GenerateContentResponse`] in each SSE event.
/// Text parts are incremental — each chunk contains only the new fragment.
///
/// # Errors
///
/// - [`Error::Json`] — the `data:` payload is non-empty but fails to deserialize
///   as a [`GenerateContentResponse`] (e.g. malformed JSON or unexpected schema).
/// - [`Error::Stream`] — the underlying byte stream or SSE framing layer
///   produced a transport-level error.
pub fn parse_sse_stream<S>(
    byte_stream: S,
) -> impl Stream<Item = Result<GenerateContentResponse, Error>> + Send
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

                match serde_json::from_str::<GenerateContentResponse>(data) {
                    Ok(response) => Some(Ok(response)),
                    Err(e) => Some(Err(Error::Json(e))),
                }
            }
            Err(e) => Some(Err(Error::stream(e))),
        }
    })
}
