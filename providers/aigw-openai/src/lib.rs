#![forbid(unsafe_code)]

pub mod client;
pub mod error;
pub mod sse;
pub mod translate;
pub mod transport;
pub mod wire_types;

pub use client::{
    OpenAIChatCompletionStream, OpenAIClient, OpenAIResponse, OpenAIResponseStream, RequestOptions,
    ResponseMeta,
};
pub use error::{OpenAIApiError, OpenAIApiErrorKind, OpenAIError};
pub use sse::{OpenAISseError, OpenAISseStream, parse_openai_sse};
pub use translate::{
    OpenAIRequestTranslator, OpenAIResponseTranslator, OpenAIStreamParser, ResponsesRequestConfig,
    ResponsesRequestTranslator, ResponsesResponseTranslator, ResponsesStreamParser, SystemHandling,
};
pub use transport::{
    DEFAULT_OPENAI_BASE_URL, DEFAULT_TIMEOUT_SECONDS, HttpTransportConfig, OpenAIAuthConfig,
    OpenAITransport, OpenAITransportConfig, OpenAITransportConfigError, OpenAITransportRequest,
};
