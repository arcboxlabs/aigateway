use std::error::Error;
use std::fmt::{self, Display, Formatter};

use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenAIApiErrorKind {
    BadRequest,
    Authentication,
    PermissionDenied,
    NotFound,
    Conflict,
    UnprocessableEntity,
    RateLimit,
    InternalServerError,
    UnexpectedStatus,
}

impl OpenAIApiErrorKind {
    pub fn from_status(status: u16) -> Self {
        match status {
            400 => Self::BadRequest,
            401 => Self::Authentication,
            403 => Self::PermissionDenied,
            404 => Self::NotFound,
            409 => Self::Conflict,
            422 => Self::UnprocessableEntity,
            429 => Self::RateLimit,
            500..=599 => Self::InternalServerError,
            _ => Self::UnexpectedStatus,
        }
    }
}

impl Display for OpenAIApiErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadRequest => f.write_str("bad request"),
            Self::Authentication => f.write_str("authentication error"),
            Self::PermissionDenied => f.write_str("permission denied"),
            Self::NotFound => f.write_str("not found"),
            Self::Conflict => f.write_str("conflict"),
            Self::UnprocessableEntity => f.write_str("unprocessable entity"),
            Self::RateLimit => f.write_str("rate limit"),
            Self::InternalServerError => f.write_str("internal server error"),
            Self::UnexpectedStatus => f.write_str("unexpected status"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpenAIApiError {
    pub kind: OpenAIApiErrorKind,
    pub status: u16,
    pub message: String,
    pub error_type: Option<String>,
    pub param: Option<Value>,
    pub code: Option<Value>,
    pub request_id: Option<String>,
    pub body: String,
}

impl Display for OpenAIApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({}): {}", self.kind, self.status, self.message)
    }
}

impl Error for OpenAIApiError {}

#[derive(Debug, Error)]
pub enum OpenAIError {
    #[error(transparent)]
    Api(#[from] OpenAIApiError),
    #[error("invalid header `{name}` with value `{value}`")]
    InvalidHeader { name: String, value: String },
    #[error("connection error: {0}")]
    Connection(#[source] reqwest::Error),
    #[error("timeout error: {0}")]
    Timeout(#[source] reqwest::Error),
    #[error("request error: {0}")]
    Request(#[source] reqwest::Error),
    #[error("failed to decode response body: {source}")]
    Decode {
        #[source]
        source: serde_json::Error,
        body: String,
    },
    #[error(transparent)]
    Sse(#[from] crate::sse::OpenAISseError),
}

impl OpenAIError {
    pub fn from_reqwest(error: reqwest::Error) -> Self {
        if error.is_timeout() {
            Self::Timeout(error)
        } else if error.is_connect() {
            Self::Connection(error)
        } else {
            Self::Request(error)
        }
    }
}
