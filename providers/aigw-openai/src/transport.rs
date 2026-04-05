use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};

use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use serde::de::Deserializer;

pub const DEFAULT_OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 600;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct HttpTransportConfig {
    #[serde(default = "default_openai_base_url")]
    pub base_url: String,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
    #[serde(default)]
    pub default_headers: BTreeMap<String, String>,
}

impl HttpTransportConfig {
    pub fn normalize(&mut self) -> Result<(), OpenAITransportConfigError> {
        self.base_url = self.base_url.trim().trim_end_matches('/').to_owned();

        if self.base_url.is_empty() {
            return Err(OpenAITransportConfigError::MissingBaseUrl);
        }

        if !(self.base_url.starts_with("http://") || self.base_url.starts_with("https://")) {
            return Err(OpenAITransportConfigError::InvalidBaseUrl(
                self.base_url.clone(),
            ));
        }

        if self.timeout_seconds == 0 {
            return Err(OpenAITransportConfigError::InvalidTimeoutSeconds(
                self.timeout_seconds,
            ));
        }

        Ok(())
    }
}

#[derive(Clone, Deserialize)]
pub struct OpenAIAuthConfig {
    #[serde(deserialize_with = "deserialize_secret_string")]
    pub api_key: SecretString,
    #[serde(default)]
    pub organization: Option<String>,
    #[serde(default)]
    pub project: Option<String>,
}

impl OpenAIAuthConfig {
    pub fn normalize(&mut self) -> Result<(), OpenAITransportConfigError> {
        let trimmed = self.api_key.expose_secret().trim().to_owned();
        if trimmed.is_empty() {
            return Err(OpenAITransportConfigError::MissingApiKey);
        }
        self.api_key = SecretString::from(trimmed);

        self.organization = self.organization.take().and_then(normalize_optional_field);
        self.project = self.project.take().and_then(normalize_optional_field);

        Ok(())
    }
}

impl Debug for OpenAIAuthConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenAIAuthConfig")
            .field("api_key", &"[REDACTED]")
            .field("organization", &self.organization)
            .field("project", &self.project)
            .finish()
    }
}

#[derive(Clone, Deserialize)]
pub struct OpenAITransportConfig {
    #[serde(flatten)]
    pub http: HttpTransportConfig,
    #[serde(flatten)]
    pub auth: OpenAIAuthConfig,
}

impl OpenAITransportConfig {
    pub fn normalize(&mut self) -> Result<(), OpenAITransportConfigError> {
        self.http.normalize()?;
        self.auth.normalize()?;
        Ok(())
    }

    pub fn base_url(&self) -> &str {
        &self.http.base_url
    }

    pub fn api_key(&self) -> &SecretString {
        &self.auth.api_key
    }

    pub fn organization(&self) -> Option<&str> {
        self.auth.organization.as_deref()
    }

    pub fn project(&self) -> Option<&str> {
        self.auth.project.as_deref()
    }

    pub fn default_headers(&self) -> &BTreeMap<String, String> {
        &self.http.default_headers
    }

    pub fn timeout_seconds(&self) -> u64 {
        self.http.timeout_seconds
    }
}

impl Debug for OpenAITransportConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenAITransportConfig")
            .field("base_url", &self.http.base_url)
            .field("timeout_seconds", &self.http.timeout_seconds)
            .field("api_key", &"[REDACTED]")
            .field("organization", &self.auth.organization)
            .field("project", &self.auth.project)
            .field("default_headers", &self.http.default_headers)
            .finish()
    }
}

#[derive(Clone)]
pub struct OpenAITransport {
    config: OpenAITransportConfig,
}

impl OpenAITransport {
    pub fn new(mut config: OpenAITransportConfig) -> Result<Self, OpenAITransportConfigError> {
        config.normalize()?;
        Ok(Self { config })
    }

    pub fn base_url(&self) -> &str {
        self.config.base_url()
    }

    pub fn api_key(&self) -> &SecretString {
        self.config.api_key()
    }

    pub fn organization(&self) -> Option<&str> {
        self.config.organization()
    }

    pub fn project(&self) -> Option<&str> {
        self.config.project()
    }

    pub fn default_headers(&self) -> &BTreeMap<String, String> {
        self.config.default_headers()
    }

    pub fn timeout_seconds(&self) -> u64 {
        self.config.timeout_seconds()
    }

    pub fn prepare_request(
        &self,
        path: &str,
        extra_headers: &BTreeMap<String, String>,
    ) -> OpenAITransportRequest {
        let mut headers = self.default_headers().clone();

        for (key, value) in extra_headers {
            headers.insert(key.clone(), value.clone());
        }

        headers.insert(
            "Authorization".to_owned(),
            format!("Bearer {}", self.api_key().expose_secret()),
        );

        if let Some(organization) = self.organization() {
            headers.insert("OpenAI-Organization".to_owned(), organization.to_owned());
        }

        if let Some(project) = self.project() {
            headers.insert("OpenAI-Project".to_owned(), project.to_owned());
        }

        OpenAITransportRequest {
            url: join_url(self.base_url(), path),
            headers,
        }
    }

    pub fn prepare_json_request(
        &self,
        path: &str,
        extra_headers: &BTreeMap<String, String>,
    ) -> OpenAITransportRequest {
        let mut json_headers = extra_headers.clone();
        json_headers
            .entry("Accept".to_owned())
            .or_insert_with(|| "application/json".to_owned());
        json_headers
            .entry("Content-Type".to_owned())
            .or_insert_with(|| "application/json".to_owned());

        self.prepare_request(path, &json_headers)
    }
}

impl Debug for OpenAITransport {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenAITransport")
            .field("config", &self.config)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAITransportRequest {
    pub url: String,
    pub headers: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenAITransportConfigError {
    MissingBaseUrl,
    MissingApiKey,
    InvalidBaseUrl(String),
    InvalidTimeoutSeconds(u64),
}

impl Display for OpenAITransportConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingBaseUrl => f.write_str("openai transport base_url is required"),
            Self::MissingApiKey => f.write_str("openai transport api_key is required"),
            Self::InvalidBaseUrl(base_url) => {
                write!(f, "openai transport base_url is invalid: {base_url}")
            }
            Self::InvalidTimeoutSeconds(timeout_seconds) => {
                write!(
                    f,
                    "openai transport timeout_seconds must be greater than zero: {timeout_seconds}"
                )
            }
        }
    }
}

impl Error for OpenAITransportConfigError {}

fn default_openai_base_url() -> String {
    DEFAULT_OPENAI_BASE_URL.to_owned()
}

fn default_timeout_seconds() -> u64 {
    DEFAULT_TIMEOUT_SECONDS
}

fn deserialize_secret_string<'de, D>(deserializer: D) -> Result<SecretString, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(SecretString::from(s))
}

fn normalize_optional_field(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn join_url(base_url: &str, path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return base_url.to_owned();
    }

    format!("{base_url}/{}", trimmed.trim_start_matches('/'))
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_OPENAI_BASE_URL, DEFAULT_TIMEOUT_SECONDS, HttpTransportConfig, OpenAIAuthConfig,
        OpenAITransport, OpenAITransportConfig,
    };
    use secrecy::SecretString;
    use std::collections::BTreeMap;

    fn config() -> OpenAITransportConfig {
        OpenAITransportConfig {
            http: HttpTransportConfig {
                base_url: " https://api.openai.com/v1/ ".to_owned(),
                timeout_seconds: 30,
                default_headers: BTreeMap::from([("X-Default".to_owned(), "default".to_owned())]),
            },
            auth: OpenAIAuthConfig {
                api_key: SecretString::from(" test-key ".to_owned()),
                organization: Some(" org_123 ".to_owned()),
                project: Some(" proj_456 ".to_owned()),
            },
        }
    }

    #[test]
    fn minimal_config_uses_default_base_url() {
        let json = r#"{
            "api_key": "sk-test"
        }"#;

        let config: OpenAITransportConfig = serde_json::from_str(json).unwrap();
        let transport = OpenAITransport::new(config).unwrap();
        assert_eq!(transport.base_url(), DEFAULT_OPENAI_BASE_URL);
        assert_eq!(transport.timeout_seconds(), DEFAULT_TIMEOUT_SECONDS);
    }

    #[test]
    fn debug_redacts_api_key() {
        let transport = OpenAITransport::new(config()).unwrap();
        let debug = format!("{transport:?}");
        assert!(
            !debug.contains("test-key"),
            "api_key leaked in Debug output"
        );

        let config = config();
        let debug = format!("{config:?}");
        assert!(
            !debug.contains("test-key"),
            "api_key leaked in Debug output"
        );
    }

    #[test]
    fn prepare_json_request_normalizes_and_merges_headers() {
        let transport = OpenAITransport::new(config()).unwrap();
        let request = transport.prepare_json_request(
            "/chat/completions",
            &BTreeMap::from([("X-Request".to_owned(), "request".to_owned())]),
        );

        assert_eq!(request.url, "https://api.openai.com/v1/chat/completions");
        assert_eq!(
            request.headers.get("Authorization"),
            Some(&"Bearer test-key".to_owned())
        );
        assert_eq!(
            request.headers.get("OpenAI-Organization"),
            Some(&"org_123".to_owned())
        );
        assert_eq!(
            request.headers.get("OpenAI-Project"),
            Some(&"proj_456".to_owned())
        );
        assert_eq!(
            request.headers.get("X-Default"),
            Some(&"default".to_owned())
        );
        assert_eq!(
            request.headers.get("X-Request"),
            Some(&"request".to_owned())
        );
    }

    #[test]
    fn prepare_request_does_not_allow_overriding_auth_headers() {
        let mut config = config();
        config.http.default_headers.insert(
            "Authorization".to_owned(),
            "Bearer default-override".to_owned(),
        );

        let transport = OpenAITransport::new(config).unwrap();
        let request = transport.prepare_request(
            "/models",
            &BTreeMap::from([
                (
                    "Authorization".to_owned(),
                    "Bearer request-override".to_owned(),
                ),
                ("OpenAI-Organization".to_owned(), "org_override".to_owned()),
                ("OpenAI-Project".to_owned(), "proj_override".to_owned()),
            ]),
        );

        assert_eq!(
            request.headers.get("Authorization"),
            Some(&"Bearer test-key".to_owned())
        );
        assert_eq!(
            request.headers.get("OpenAI-Organization"),
            Some(&"org_123".to_owned())
        );
        assert_eq!(
            request.headers.get("OpenAI-Project"),
            Some(&"proj_456".to_owned())
        );
    }

    #[test]
    fn new_rejects_zero_timeout() {
        let mut config = config();
        config.http.timeout_seconds = 0;

        let err = OpenAITransport::new(config).unwrap_err();
        assert_eq!(
            err,
            super::OpenAITransportConfigError::InvalidTimeoutSeconds(0)
        );
    }
}
