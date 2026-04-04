//! HTTP client for the Anthropic Messages API.

use std::time::Duration;

use bytes::Bytes;
use futures::StreamExt;
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use secrecy::{ExposeSecret, SecretString};
use tokio_stream::Stream;

use crate::error::Error;
use crate::streaming::parse_sse_stream;
use crate::types::{ApiErrorResponse, MessagesRequest, MessagesResponse, StreamEvent};

const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";
const DEFAULT_VERSION: &str = "2023-06-01";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(600);

/// Configuration for [`Client`].
///
/// The API key is stored as [`SecretString`] and will never appear in Debug output or logs.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub api_key: SecretString,
    pub base_url: String,
    pub version: String,
    /// Request timeout. Defaults to 600s (10 min) to accommodate long-running completions.
    pub timeout: Duration,
}

impl ClientConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: SecretString::from(api_key.into()),
            base_url: DEFAULT_BASE_URL.to_owned(),
            version: DEFAULT_VERSION.to_owned(),
            timeout: DEFAULT_TIMEOUT,
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

/// Anthropic API client.
///
/// Headers are pre-built at construction time to avoid repeated parsing
/// and to surface invalid API key values early.
#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    headers: HeaderMap,
    messages_url: String,
}

impl Client {
    /// Create a new client. Returns an error if the API key contains invalid header characters.
    pub fn new(config: ClientConfig) -> Result<Self, Error> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(config.api_key.expose_secret())
                .map_err(|e| Error::Config(format!("invalid api key: {e}")))?,
        );
        headers.insert(
            "anthropic-version",
            HeaderValue::from_str(&config.version)
                .map_err(|e| Error::Config(format!("invalid version: {e}")))?,
        );

        let messages_url = format!("{}/v1/messages", config.base_url.trim_end_matches('/'));

        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| Error::Config(format!("failed to build HTTP client: {e}")))?;

        Ok(Self {
            http,
            headers,
            messages_url,
        })
    }

    /// Create a client from the `ANTHROPIC_API_KEY` environment variable.
    pub fn from_env() -> Result<Self, Error> {
        let api_key =
            std::env::var("ANTHROPIC_API_KEY").map_err(|e| Error::Config(e.to_string()))?;
        Self::new(ClientConfig::new(api_key))
    }

    /// Send a non-streaming messages request.
    pub async fn messages(&self, req: &MessagesRequest) -> Result<MessagesResponse, Error> {
        let response = self
            .http
            .post(&self.messages_url)
            .headers(self.headers.clone())
            .json(req)
            .send()
            .await?;

        if response.status().is_success() {
            let body = response.text().await?;
            Ok(serde_json::from_str(&body)?)
        } else {
            Err(self.parse_error_response(response).await?)
        }
    }

    /// Send a streaming messages request, returning a stream of events.
    pub async fn messages_stream(
        &self,
        req: &MessagesRequest,
    ) -> Result<impl Stream<Item = Result<StreamEvent, Error>> + Send, Error> {
        let response = self
            .http
            .post(&self.messages_url)
            .headers(self.headers.clone())
            .json(req)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await?);
        }

        let byte_stream = response.bytes_stream().map(|result| {
            result.map(Bytes::from).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::ConnectionReset, e.to_string())
            })
        });

        Ok(parse_sse_stream(byte_stream))
    }

    /// Parse an error response body. Falls back to [`Error::UnexpectedResponse`]
    /// if the body is not valid Anthropic JSON (e.g. a 502 HTML page from a proxy).
    async fn parse_error_response(&self, response: reqwest::Response) -> Result<Error, Error> {
        let status = response.status().as_u16();
        let body = response.text().await?;

        match serde_json::from_str::<ApiErrorResponse>(&body) {
            Ok(parsed) => Ok(Error::from_api_error(status, parsed.error)),
            Err(_) => Ok(Error::UnexpectedResponse { status, body }),
        }
    }
}
