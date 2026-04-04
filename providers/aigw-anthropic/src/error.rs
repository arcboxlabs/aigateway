//! Error types for the Anthropic provider.

use crate::types::ApiError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Anthropic API error ({status}): [{error_type}] {message}")]
    Api {
        status: u16,
        error_type: String,
        message: String,
    },

    /// Non-JSON error response from the server (e.g. 502 HTML page).
    #[error("unexpected error response ({status}): {body}")]
    UnexpectedResponse { status: u16, body: String },

    #[error("SSE stream error: {source}")]
    Stream {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

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
