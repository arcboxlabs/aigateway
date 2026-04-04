#![forbid(unsafe_code)]

pub mod client;
pub mod error;
pub mod sse;
pub mod transport;
pub mod wire_types;

pub use client::{OpenAIChatCompletionStream, OpenAIClient, OpenAIResponseStream};
pub use error::{OpenAIApiError, OpenAIApiErrorKind, OpenAIError};
pub use sse::{OpenAISseError, OpenAISseStream, parse_openai_sse};
pub use transport::{
    DEFAULT_OPENAI_BASE_URL, DEFAULT_TIMEOUT_SECONDS, HttpTransportConfig, OpenAIAuthConfig,
    OpenAITransport, OpenAITransportConfig, OpenAITransportConfigError, OpenAITransportRequest,
};
