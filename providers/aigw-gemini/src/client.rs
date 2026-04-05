//! HTTP client for the Google Gemini API.

use std::time::Duration;

use bon::Builder;
use futures::StreamExt;
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use secrecy::{ExposeSecret, SecretString};
use tokio_stream::Stream;

use crate::error::Error;
use crate::streaming::parse_sse_stream;
use crate::types::{GenerateContentRequest, GenerateContentResponse, GoogleErrorResponse};

const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com";
const DEFAULT_API_VERSION: &str = "v1beta";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(600);

/// Configuration for [`Client`].
///
/// The API key is stored as [`SecretString`] and will never appear in Debug output or logs.
///
/// Call [`normalize()`](Self::normalize) (or let [`Client::new`] do it) to trim
/// whitespace and trailing slashes from URLs.
#[derive(Debug, Clone, Builder)]
#[builder(on(String, into))]
pub struct ClientConfig {
    /// Google AI API key. Accepts `String` or `&str` via `Into<SecretString>`.
    #[builder(into)]
    pub api_key: SecretString,
    /// Base URL for the API. Defaults to `https://generativelanguage.googleapis.com`.
    #[builder(default = DEFAULT_BASE_URL.to_owned())]
    pub base_url: String,
    /// API version path segment. Defaults to `"v1beta"`.
    #[builder(default = DEFAULT_API_VERSION.to_owned())]
    pub api_version: String,
    /// Request timeout. Defaults to 600s (10 min) to accommodate long-running completions.
    #[builder(default = DEFAULT_TIMEOUT)]
    pub timeout: Duration,
}

impl ClientConfig {
    /// Normalize configuration values: trim whitespace, strip trailing slashes.
    pub fn normalize(&mut self) {
        self.base_url = self.base_url.trim().trim_end_matches('/').to_owned();
        self.api_version = self.api_version.trim().trim_end_matches('/').to_owned();
    }
}

/// Google Gemini API client.
///
/// Headers are pre-built at construction time to avoid repeated parsing
/// and to surface invalid API key values early.
#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    headers: HeaderMap,
    base_url: String,
    api_version: String,
}

impl Client {
    /// Create a new client. Returns an error if the API key contains invalid header characters.
    ///
    /// Calls [`ClientConfig::normalize`] before using the configuration.
    pub fn new(mut config: ClientConfig) -> Result<Self, Error> {
        config.normalize();

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "x-goog-api-key",
            HeaderValue::from_str(config.api_key.expose_secret())
                .map_err(|e| Error::Config(format!("invalid api key: {e}")))?,
        );

        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| Error::Config(format!("failed to build HTTP client: {e}")))?;

        Ok(Self {
            http,
            headers,
            base_url: config.base_url,
            api_version: config.api_version,
        })
    }

    /// Create a client from the `GOOGLE_API_KEY` environment variable.
    pub fn from_env() -> Result<Self, Error> {
        let api_key = std::env::var("GOOGLE_API_KEY").map_err(|e| Error::Config(e.to_string()))?;
        Self::new(ClientConfig::builder().api_key(api_key).build())
    }

    /// Send a non-streaming generateContent request.
    ///
    /// The model from [`GenerateContentRequest::model`] is placed in the URL path;
    /// it is not serialized in the JSON body.
    pub async fn generate_content(
        &self,
        req: &GenerateContentRequest,
    ) -> Result<GenerateContentResponse, Error> {
        let url = format!(
            "{}/{}/models/{}:generateContent",
            self.base_url, self.api_version, req.model
        );

        let response = self
            .http
            .post(&url)
            .headers(self.headers.clone())
            .json(req)
            .send()
            .await?;

        if response.status().is_success() {
            let body = response.text().await?;
            Ok(serde_json::from_str(&body)?)
        } else {
            Err(self.parse_error_response(response).await)
        }
    }

    /// Send a streaming generateContent request, returning a stream of responses.
    ///
    /// Each SSE event is a complete [`GenerateContentResponse`] containing
    /// incremental text parts. The stream completes when the connection closes.
    pub async fn generate_content_stream(
        &self,
        req: &GenerateContentRequest,
    ) -> Result<impl Stream<Item = Result<GenerateContentResponse, Error>> + Send, Error> {
        let url = format!(
            "{}/{}/models/{}:streamGenerateContent?alt=sse",
            self.base_url, self.api_version, req.model
        );

        let response = self
            .http
            .post(&url)
            .headers(self.headers.clone())
            .json(req)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        let byte_stream = response.bytes_stream().map(|result| {
            result.map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::ConnectionReset, e.to_string())
            })
        });

        Ok(parse_sse_stream(byte_stream))
    }

    /// Parse an error response body. Falls back to [`Error::UnexpectedResponse`]
    /// if the body is not valid Google API JSON (e.g. a 502 HTML page from a proxy).
    async fn parse_error_response(&self, response: reqwest::Response) -> Error {
        let status = response.status().as_u16();
        let body = match response.text().await {
            Ok(body) => body,
            Err(e) => return Error::Http(e),
        };

        match serde_json::from_str::<GoogleErrorResponse>(&body) {
            Ok(parsed) => Error::from_api_error(status, parsed.error),
            Err(_) => Error::UnexpectedResponse { status, body },
        }
    }
}
