use std::fmt::Display;
use std::pin::Pin;
use std::task::{Context, Poll};

use eventsource_stream::{EventStreamError, Eventsource};
use futures_core::Stream;
use futures_util::StreamExt;
use serde::de::DeserializeOwned;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum OpenAISseError {
    #[error("SSE transport error: {0}")]
    Transport(String),
    #[error("SSE parse error: {0}")]
    Parse(String),
    #[error("SSE JSON decode error: {message}; data: {data}")]
    JsonDecode { data: String, message: String },
}

enum OpenAISseFrame<T> {
    Message(T),
    Done,
}

pub struct OpenAISseStream<T> {
    inner: Pin<Box<dyn Stream<Item = Result<OpenAISseFrame<T>, OpenAISseError>> + Send>>,
    done: bool,
}

impl<T> OpenAISseStream<T> {
    fn new(
        inner: Pin<Box<dyn Stream<Item = Result<OpenAISseFrame<T>, OpenAISseError>> + Send>>,
    ) -> Self {
        Self { inner, done: false }
    }
}

impl<T> Stream for OpenAISseStream<T> {
    type Item = Result<T, OpenAISseError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.done {
            return Poll::Ready(None);
        }

        match self.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(OpenAISseFrame::Message(value)))) => Poll::Ready(Some(Ok(value))),
            Poll::Ready(Some(Ok(OpenAISseFrame::Done))) => {
                self.done = true;
                Poll::Ready(None)
            }
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(err))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub fn parse_openai_sse<S, B, E, T>(stream: S) -> OpenAISseStream<T>
where
    S: Stream<Item = Result<B, E>> + Send + 'static,
    B: AsRef<[u8]> + Send + 'static,
    E: Display + Send + Sync + 'static,
    T: DeserializeOwned + Send + 'static,
{
    let inner = stream.eventsource().map(|event| match event {
        Ok(event) => parse_openai_sse_event(event.data),
        Err(err) => Err(map_eventsource_error(err)),
    });

    OpenAISseStream::new(Box::pin(inner))
}

fn parse_openai_sse_event<T>(data: String) -> Result<OpenAISseFrame<T>, OpenAISseError>
where
    T: DeserializeOwned,
{
    if data == "[DONE]" {
        return Ok(OpenAISseFrame::Done);
    }

    serde_json::from_str(&data)
        .map(OpenAISseFrame::Message)
        .map_err(|error| OpenAISseError::JsonDecode {
            data,
            message: error.to_string(),
        })
}

fn map_eventsource_error<E>(error: EventStreamError<E>) -> OpenAISseError
where
    E: Display,
{
    match error {
        EventStreamError::Transport(error) => OpenAISseError::Transport(error.to_string()),
        EventStreamError::Utf8(error) => OpenAISseError::Parse(error.to_string()),
        EventStreamError::Parser(error) => OpenAISseError::Parse(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use futures_util::{StreamExt, stream};

    use super::parse_openai_sse;

    #[tokio::test]
    async fn parse_openai_sse_stops_at_done() {
        let source = stream::iter(vec![Ok::<_, std::io::Error>(Bytes::from(
            "data: {\"id\":\"a\",\"value\":1}\n\n\
             data: [DONE]\n\n\
             data: {\"id\":\"b\",\"value\":2}\n\n",
        ))]);

        let events = parse_openai_sse::<_, _, _, serde_json::Value>(source)
            .collect::<Vec<_>>()
            .await;

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].as_ref().unwrap()["id"], "a");
    }
}
