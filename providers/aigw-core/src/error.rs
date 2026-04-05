//! Error types for the translation layer.

use std::time::Duration;

/// Errors that occur during request/response translation.
///
/// These are problems with the **translation process itself** — incompatible
/// content, missing required fields, etc. They are distinct from
/// [`ProviderError`], which represents errors returned by upstream APIs.
#[derive(Debug, thiserror::Error)]
pub enum TranslateError {
    /// A required field is missing and cannot be defaulted.
    /// E.g., Anthropic requires `max_tokens` but the canonical request omits it.
    #[error("missing required field: {field}")]
    MissingField { field: &'static str },

    /// The request uses a feature the target provider doesn't support.
    /// E.g., `n > 1` targeting Anthropic, or `response_format: json_schema`
    /// targeting a provider without structured output.
    #[error("unsupported feature for provider {provider}: {feature}")]
    UnsupportedFeature {
        provider: &'static str,
        feature: String,
    },

    /// Content type incompatibility.
    /// E.g., image content targeting a text-only provider.
    #[error("incompatible content: {reason}")]
    IncompatibleContent { reason: String },

    /// JSON serialization/deserialization failed during translation.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// Stream parsing failed — malformed SSE data, unexpected event structure, etc.
    #[error("stream parse error: {message}")]
    StreamParse { message: String },

    /// Catch-all for translation errors not covered above.
    #[error("{0}")]
    Other(String),
}

/// Errors returned by upstream provider APIs.
///
/// Translators produce these from raw HTTP error responses. The gateway
/// uses them for retry decisions, error forwarding, and observability.
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    /// Rate limited (HTTP 429).
    #[error("rate limited: {message}")]
    RateLimited {
        /// Retry-After duration, if the provider specified one.
        retry_after: Option<Duration>,
        message: String,
    },

    /// Authentication failed (HTTP 401).
    #[error("authentication failed: {message}")]
    AuthenticationFailed { message: String },

    /// Permission denied (HTTP 403).
    #[error("permission denied: {message}")]
    PermissionDenied { message: String },

    /// Model not found (HTTP 404).
    #[error("model not found: {model}")]
    ModelNotFound { model: String },

    /// Input too long (HTTP 400, context length exceeded).
    #[error("context length exceeded: max {max} tokens, requested {requested}")]
    ContextLengthExceeded { max: u64, requested: u64 },

    /// Invalid request (HTTP 400, other than context length).
    #[error("invalid request: {message}")]
    InvalidRequest { message: String },

    /// Provider is overloaded (HTTP 529, Anthropic-specific).
    #[error("provider overloaded: {message}")]
    Overloaded { message: String },

    /// Server error (HTTP 5xx).
    #[error("server error (HTTP {status}): {message}")]
    ServerError { status: u16, message: String },

    /// Unrecognized error — preserves raw status and body.
    #[error("unexpected error (HTTP {status}): {body}")]
    Unknown { status: u16, body: String },
}

/// Maps an HTTP status code + extracted message to a [`ProviderError`].
///
/// This is the common logic shared across all provider `translate_error` impls.
/// Provider-specific status codes (e.g. Anthropic's 529) should be handled
/// *before* calling this function.
///
/// # Arguments
/// - `status`: HTTP status code
/// - `headers`: response headers (used to extract `retry-after` for 429)
/// - `message`: error message already extracted from the provider's error body
pub fn map_error_status(status: u16, headers: &http::HeaderMap, message: String) -> ProviderError {
    match status {
        401 => ProviderError::AuthenticationFailed { message },
        403 => ProviderError::PermissionDenied { message },
        404 => ProviderError::ModelNotFound { model: message },
        429 => {
            let retry_after = headers
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.trim().parse::<u64>().ok())
                .map(Duration::from_secs);
            ProviderError::RateLimited {
                retry_after,
                message,
            }
        }
        400 => ProviderError::InvalidRequest { message },
        500..=599 => ProviderError::ServerError { status, message },
        _ => ProviderError::Unknown {
            status,
            body: message,
        },
    }
}
