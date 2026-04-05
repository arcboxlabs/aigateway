//! Translation layer: canonical ↔ OpenAI wire types.
//!
//! Since the canonical format IS the OpenAI format, request translation is
//! near-passthrough (direct serialization). Response translation deserializes
//! directly into canonical types. Only streaming and error translation require
//! explicit mapping.

mod request;
mod response;
mod stream;

pub use request::OpenAIRequestTranslator;
pub use response::OpenAIResponseTranslator;
pub use stream::OpenAIStreamParser;
