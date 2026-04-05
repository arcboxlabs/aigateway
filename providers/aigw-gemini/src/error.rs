//! Error types for the Gemini provider.

use crate::types::GoogleApiError;

/// Errors that can occur when using the Gemini API client.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// HTTP transport error (connection, TLS, timeout via reqwest).
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON serialization or deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Google API returned a structured error response.
    #[error("Gemini API error ({status}): [{code}] {message}")]
    Api {
        /// HTTP status code.
        status: u16,
        /// gRPC-style status code (e.g. `"INVALID_ARGUMENT"`, `"RESOURCE_EXHAUSTED"`).
        code: String,
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
    pub(crate) fn from_api_error(status: u16, error: GoogleApiError) -> Self {
        Self::Api {
            status,
            code: error.status,
            message: error.message,
        }
    }

    pub(crate) fn stream(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Stream {
            source: Box::new(err),
        }
    }
}
