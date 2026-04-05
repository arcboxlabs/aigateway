//! Umbrella crate for AI Gateway provider clients.
//!
//! Re-exports all `aigw-*` provider crates behind feature flags as namespaced modules.
//! All features are enabled by default; disable with `default-features = false`
//! and pick only the providers you need.
//!
//! ```toml
//! # All providers (default)
//! aigw = "0.1"
//!
//! # Only Anthropic + OpenAI
//! aigw = { version = "0.1", default-features = false, features = ["anthropic", "openai"] }
//! ```
//!
//! Usage: `aigw::openai::OpenAIClient`, `aigw::anthropic::Client`, etc.

#[cfg(feature = "openai")]
pub use aigw_openai as openai;

#[cfg(feature = "openai-compat")]
pub use aigw_openai_compat as openai_compat;

#[cfg(feature = "anthropic")]
pub use aigw_anthropic as anthropic;

#[cfg(feature = "gemini")]
pub use aigw_gemini as gemini;
