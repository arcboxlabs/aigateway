//! Error types for the Anthropic provider.

use crate::types::ApiError;

/// Errors that can occur when using the Anthropic API client.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// HTTP transport error (connection, TLS, timeout via reqwest).
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON serialization or deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Anthropic API returned a structured error response.
    #[error("Anthropic API error ({status}): [{error_type}] {message}")]
    Api {
        /// HTTP status code.
        status: u16,
        /// Error type (e.g. `"invalid_request_error"`, `"rate_limit_error"`).
        error_type: String,
        /// Human-readable error message.
        message: String,
    },

    /// Non-JSON error response from the server (e.g. 502 HTML page from a proxy).
    #[error("unexpected error response ({status}): {body}")]
    UnexpectedResponse {
        /// HTTP status code.
        status: u16,
        /// Raw response body.
        body: String,
    },

    /// Error from the SSE stream parser.
    #[error("SSE stream error: {source}")]
    Stream {
        /// The underlying stream error.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Invalid client configuration.
    #[error("invalid configuration: {0}")]
    Config(String),
}

impl Error {
    pub(crate) fn from_api_error(status: u16, error: ApiError) -> Self {
        Self::Api {
            status,
            error_type: error.r#type,
            message: error.message,
        }
    }

    pub(crate) fn stream(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Stream {
            source: Box::new(err),
        }
    }
}
