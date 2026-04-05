//! HTTP transport layer for the Anthropic API.
//!
//! Handles configuration validation, header construction, and URL assembly.
//! The [`Client`](crate::Client) delegates all HTTP plumbing to this layer
//! and focuses on API-level logic.

use std::fmt;
use std::time::Duration;

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use secrecy::{ExposeSecret, SecretString};

const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";
const DEFAULT_VERSION: &str = "2023-06-01";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(600);

/// Authentication mode for the Anthropic API.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum AuthMode {
    /// `x-api-key` header (default Anthropic authentication).
    #[default]
    ApiKey,
    /// `Authorization: Bearer` header (OAuth / passthrough).
    Bearer,
}

/// Configuration for the Anthropic HTTP transport.
///
/// The API key is stored as [`SecretString`] and will never appear in Debug output.
#[derive(Clone)]
pub struct TransportConfig {
    /// Anthropic API key or OAuth token.
    pub api_key: SecretString,
    /// How to send the credential. Defaults to [`AuthMode::ApiKey`] (`x-api-key` header).
    pub auth_mode: AuthMode,
    /// Base URL for the API. Defaults to `https://api.anthropic.com`.
    pub base_url: String,
    /// API version header (`anthropic-version`). Defaults to `"2023-06-01"`.
    pub version: String,
    /// Request timeout. Defaults to 600s.
    pub timeout: Duration,
    /// Value for the `anthropic-beta` header (comma-separated beta feature flags).
    pub beta: Option<String>,
    /// Additional headers to include in every request (e.g. `User-Agent`, `X-App`).
    pub extra_headers: HeaderMap,
}

impl TransportConfig {
    /// Validate and normalize the configuration.
    fn normalize(&mut self) -> Result<(), TransportConfigError> {
        let trimmed_key = self.api_key.expose_secret().trim().to_owned();
        if trimmed_key.is_empty() {
            return Err(TransportConfigError::MissingApiKey);
        }
        self.api_key = SecretString::from(trimmed_key);

        self.base_url = self.base_url.trim().trim_end_matches('/').to_owned();
        if self.base_url.is_empty() {
            return Err(TransportConfigError::MissingBaseUrl);
        }
        if !(self.base_url.starts_with("http://") || self.base_url.starts_with("https://")) {
            return Err(TransportConfigError::InvalidBaseUrl(self.base_url.clone()));
        }

        if self.timeout.is_zero() {
            return Err(TransportConfigError::InvalidTimeout);
        }

        Ok(())
    }
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            api_key: SecretString::from(String::new()),
            auth_mode: AuthMode::default(),
            base_url: DEFAULT_BASE_URL.to_owned(),
            version: DEFAULT_VERSION.to_owned(),
            timeout: DEFAULT_TIMEOUT,
            beta: None,
            extra_headers: HeaderMap::new(),
        }
    }
}

impl fmt::Debug for TransportConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransportConfig")
            .field("api_key", &"[REDACTED]")
            .field("auth_mode", &self.auth_mode)
            .field("base_url", &self.base_url)
            .field("version", &self.version)
            .field("timeout", &self.timeout)
            .field("beta", &self.beta)
            .field("extra_headers", &self.extra_headers)
            .finish()
    }
}

/// Validated Anthropic HTTP transport.
///
/// Constructed via [`Transport::new`], which normalizes and validates the config.
/// Pre-builds the default [`HeaderMap`] so it can be cloned cheaply per request.
#[derive(Clone)]
pub struct Transport {
    base_url: String,
    timeout: Duration,
    headers: HeaderMap,
}

impl Transport {
    /// Create a new transport, validating the config.
    pub fn new(mut config: TransportConfig) -> Result<Self, TransportConfigError> {
        config.normalize()?;

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        match config.auth_mode {
            AuthMode::ApiKey => {
                headers.insert(
                    "x-api-key",
                    HeaderValue::from_str(config.api_key.expose_secret())
                        .map_err(|_| TransportConfigError::InvalidApiKey)?,
                );
            }
            AuthMode::Bearer => {
                headers.insert(
                    AUTHORIZATION,
                    HeaderValue::from_str(&format!("Bearer {}", config.api_key.expose_secret()))
                        .map_err(|_| TransportConfigError::InvalidApiKey)?,
                );
            }
        }
        headers.insert(
            "anthropic-version",
            HeaderValue::from_str(&config.version)
                .map_err(|_| TransportConfigError::InvalidVersion(config.version.clone()))?,
        );

        if let Some(beta) = &config.beta {
            headers.insert(
                "anthropic-beta",
                HeaderValue::from_str(beta)
                    .map_err(|_| TransportConfigError::InvalidBeta(beta.clone()))?,
            );
        }

        headers.extend(config.extra_headers);

        Ok(Self {
            base_url: config.base_url,
            timeout: config.timeout,
            headers,
        })
    }

    /// Build a full URL for the given API path.
    pub fn url(&self, path: &str) -> String {
        let path = path.trim();
        if path.is_empty() {
            return self.base_url.clone();
        }
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }

    /// Pre-built default headers (cloned per request).
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Configured request timeout.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Base URL (normalized, no trailing slash).
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

impl fmt::Debug for Transport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Transport")
            .field("base_url", &self.base_url)
            .field("timeout", &self.timeout)
            .finish_non_exhaustive()
    }
}

/// Transport configuration error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportConfigError {
    MissingApiKey,
    MissingBaseUrl,
    InvalidApiKey,
    InvalidBaseUrl(String),
    InvalidVersion(String),
    InvalidBeta(String),
    InvalidTimeout,
}

impl fmt::Display for TransportConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingApiKey => f.write_str("api_key is required"),
            Self::MissingBaseUrl => f.write_str("base_url is required"),
            Self::InvalidApiKey => f.write_str("api_key contains invalid header characters"),
            Self::InvalidBaseUrl(url) => write!(f, "invalid base_url: {url}"),
            Self::InvalidVersion(v) => write!(f, "invalid anthropic-version: {v}"),
            Self::InvalidBeta(b) => write!(f, "invalid anthropic-beta: {b}"),
            Self::InvalidTimeout => f.write_str("timeout must be greater than zero"),
        }
    }
}

impl std::error::Error for TransportConfigError {}

#[cfg(test)]
mod tests {
    use secrecy::SecretString;

    use super::{Transport, TransportConfig, TransportConfigError};

    fn config() -> TransportConfig {
        TransportConfig {
            api_key: SecretString::from("sk-ant-test"),
            ..Default::default()
        }
    }

    #[test]
    fn valid_config_builds_transport() {
        let t = Transport::new(config()).unwrap();
        assert_eq!(t.base_url(), "https://api.anthropic.com");
        assert!(t.headers().contains_key("x-api-key"));
        assert!(t.headers().contains_key("anthropic-version"));
        assert!(t.headers().contains_key("content-type"));
    }

    #[test]
    fn normalizes_base_url() {
        let mut cfg = config();
        cfg.base_url = "  https://custom.api.com/  ".to_owned();
        let t = Transport::new(cfg).unwrap();
        assert_eq!(t.base_url(), "https://custom.api.com");
    }

    #[test]
    fn rejects_empty_api_key() {
        let mut cfg = config();
        cfg.api_key = SecretString::from("  ");
        assert_eq!(
            Transport::new(cfg).unwrap_err(),
            TransportConfigError::MissingApiKey,
        );
    }

    #[test]
    fn rejects_empty_base_url() {
        let mut cfg = config();
        cfg.base_url = "  ".to_owned();
        assert_eq!(
            Transport::new(cfg).unwrap_err(),
            TransportConfigError::MissingBaseUrl,
        );
    }

    #[test]
    fn rejects_non_http_base_url() {
        let mut cfg = config();
        cfg.base_url = "ftp://example.com".to_owned();
        assert_eq!(
            Transport::new(cfg).unwrap_err(),
            TransportConfigError::InvalidBaseUrl("ftp://example.com".to_owned()),
        );
    }

    #[test]
    fn rejects_zero_timeout() {
        let mut cfg = config();
        cfg.timeout = std::time::Duration::ZERO;
        assert_eq!(
            Transport::new(cfg).unwrap_err(),
            TransportConfigError::InvalidTimeout,
        );
    }

    #[test]
    fn beta_header_included_when_set() {
        let mut cfg = config();
        cfg.beta = Some("claude-code-20250219,fast-mode-2026-02-01".to_owned());
        let t = Transport::new(cfg).unwrap();
        assert!(t.headers().contains_key("anthropic-beta"));
    }

    #[test]
    fn bearer_mode_sends_authorization_header() {
        let cfg = TransportConfig {
            api_key: SecretString::from("oauth-token-123"),
            auth_mode: super::AuthMode::Bearer,
            ..Default::default()
        };
        let t = Transport::new(cfg).unwrap();
        assert!(!t.headers().contains_key("x-api-key"));
        assert_eq!(
            t.headers().get("authorization").unwrap(),
            "Bearer oauth-token-123",
        );
    }

    #[test]
    fn debug_redacts_api_key() {
        let t = Transport::new(config()).unwrap();
        let debug = format!("{t:?}");
        assert!(!debug.contains("sk-ant-test"));
    }

    #[test]
    fn url_concatenation() {
        let t = Transport::new(config()).unwrap();
        assert_eq!(
            t.url("/v1/messages"),
            "https://api.anthropic.com/v1/messages"
        );
        assert_eq!(
            t.url("v1/messages"),
            "https://api.anthropic.com/v1/messages"
        );
        assert_eq!(t.url(""), "https://api.anthropic.com");
        assert_eq!(t.url("  "), "https://api.anthropic.com");
    }
}
