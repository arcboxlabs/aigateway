//! Canonical model types for the AI Gateway.
//!
//! These types define the "standard format" that sits between clients and providers.
//! The wire format matches OpenAI Chat Completions (the de facto industry standard),
//! so inbound client requests deserialize directly into these types.
//!
//! Provider-specific fields (OpenAI's `logprobs`, Anthropic's `thinking`, Gemini's
//! `safety_settings`) flow through `extra` via `#[serde(flatten)]` — they are preserved
//! but not interpreted by the canonical model.

mod request;
mod response;
mod stream;

pub use request::*;
pub use response::*;
pub use stream::*;
