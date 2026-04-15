//! Translation layer: canonical ↔ OpenAI wire types.
//!
//! Two translation paths are provided:
//!
//! - **Chat Completions** (`OpenAI{Request,Response}Translator`, `OpenAIStreamParser`)
//!   Near-passthrough — the canonical format IS the OpenAI Chat Completions format.
//!
//! - **Responses API** (`Responses{Request,Response}Translator`, `ResponsesStreamParser`)
//!   Translates between the canonical Chat Completions format and the OpenAI
//!   Responses API format (`/v1/responses`). Messages → input items, system →
//!   instructions, tool definitions restructured.

mod request;
mod response;
mod responses_request;
mod responses_response;
pub(crate) mod responses_stream;
mod stream;

pub use request::OpenAIRequestTranslator;
pub use response::OpenAIResponseTranslator;
pub use responses_request::{ResponsesRequestConfig, ResponsesRequestTranslator, SystemHandling};
pub use responses_response::ResponsesResponseTranslator;
pub use responses_stream::ResponsesStreamParser;
pub use stream::OpenAIStreamParser;
