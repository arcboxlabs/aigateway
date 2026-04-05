//! Umbrella crate for AI Gateway provider clients.
//!
//! Re-exports all `aigw-*` provider crates behind feature flags.
//! All features are enabled by default; disable with `default-features = false`
//! and pick only the providers you need.
//!
//! ```toml
//! # All providers (default)
//! aigw = "0.0.1"
//!
//! # Only Anthropic + OpenAI
//! aigw = { version = "0.0.1", default-features = false, features = ["anthropic", "openai"] }
//! ```

#[cfg(feature = "openai")]
pub mod openai {
    //! OpenAI provider — [`aigw_openai`].
    pub use aigw_openai::*;
}

#[cfg(feature = "openai-compat")]
pub mod openai_compat {
    //! OpenAI-compatible providers (Groq, Together, vLLM, etc.) — [`aigw_openai_compat`].
    pub use aigw_openai_compat::*;
}

#[cfg(feature = "anthropic")]
pub mod anthropic {
    //! Anthropic provider — [`aigw_anthropic`].
    pub use aigw_anthropic::*;
}

#[cfg(feature = "gemini")]
pub mod gemini {
    //! Google Gemini provider — [`aigw_gemini`].
    pub use aigw_gemini::*;
}
