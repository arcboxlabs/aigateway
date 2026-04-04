use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};

use aigw_openai::{HttpTransportConfig, OpenAIAuthConfig, OpenAITransportConfigError};
use serde::Deserialize;

#[derive(Clone, PartialEq, Eq)]
pub struct OpenAICompatProvider {
    config: OpenAICompatConfig,
}

impl OpenAICompatProvider {
    pub fn new(mut config: OpenAICompatConfig) -> Result<Self, OpenAICompatConfigError> {
        config.normalize()?;
        Ok(Self { config })
    }

    pub fn name(&self) -> &str {
        &self.config.name
    }

    pub fn base_url(&self) -> &str {
        &self.config.http.base_url
    }

    pub fn api_key(&self) -> &str {
        &self.config.auth.api_key
    }

    pub fn organization(&self) -> Option<&str> {
        self.config.auth.organization.as_deref()
    }

    pub fn project(&self) -> Option<&str> {
        self.config.auth.project.as_deref()
    }

    pub fn quirks(&self) -> &Quirks {
        &self.config.quirks
    }

    pub fn default_headers(&self) -> &BTreeMap<String, String> {
        &self.config.http.default_headers
    }
}

impl Debug for OpenAICompatProvider {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenAICompatProvider")
            .field("name", &self.config.name)
            .field("base_url", &self.config.http.base_url)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct OpenAICompatConfig {
    pub name: String,
    #[serde(flatten)]
    pub http: HttpTransportConfig,
    #[serde(flatten)]
    pub auth: OpenAIAuthConfig,
    #[serde(default)]
    pub quirks: Quirks,
}

impl OpenAICompatConfig {
    fn normalize(&mut self) -> Result<(), OpenAICompatConfigError> {
        self.name = self.name.trim().to_owned();

        if self.name.is_empty() {
            return Err(OpenAICompatConfigError::MissingName);
        }

        self.http.normalize()?;
        self.auth.normalize()?;

        Ok(())
    }
}

impl Debug for OpenAICompatConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenAICompatConfig")
            .field("name", &self.name)
            .field("base_url", &self.http.base_url)
            .field("api_key", &"[REDACTED]")
            .field("organization", &self.auth.organization)
            .field("project", &self.auth.project)
            .field("default_headers", &self.http.default_headers)
            .field("quirks", &self.quirks)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub struct Quirks {
    #[serde(default = "ret_false")]
    pub supports_responses_api: bool,
    #[serde(default = "ret_true")]
    pub supports_chat_completions: bool,
    #[serde(default = "ret_true")]
    pub supports_embeddings: bool,
    #[serde(default = "ret_true")]
    pub supports_streaming: bool,
    #[serde(default = "ret_true")]
    pub supports_tool_choice: bool,
    #[serde(default = "ret_true")]
    pub supports_parallel_tool_calls: bool,
    #[serde(default = "ret_true")]
    pub supports_vision: bool,
}

fn ret_true() -> bool {
    true
}
fn ret_false() -> bool {
    false
}

impl Default for Quirks {
    fn default() -> Self {
        Self {
            supports_responses_api: false,
            supports_chat_completions: true,
            supports_embeddings: true,
            supports_streaming: true,
            supports_tool_choice: true,
            supports_parallel_tool_calls: true,
            supports_vision: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenAICompatConfigError {
    MissingName,
    MissingBaseUrl,
    MissingApiKey,
    InvalidBaseUrl(String),
    InvalidTimeoutSeconds(u64),
}

impl From<OpenAITransportConfigError> for OpenAICompatConfigError {
    fn from(value: OpenAITransportConfigError) -> Self {
        match value {
            OpenAITransportConfigError::MissingBaseUrl => Self::MissingBaseUrl,
            OpenAITransportConfigError::MissingApiKey => Self::MissingApiKey,
            OpenAITransportConfigError::InvalidBaseUrl(base_url) => Self::InvalidBaseUrl(base_url),
            OpenAITransportConfigError::InvalidTimeoutSeconds(timeout_seconds) => {
                Self::InvalidTimeoutSeconds(timeout_seconds)
            }
        }
    }
}

impl Display for OpenAICompatConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingName => f.write_str("openai-compat provider name is required"),
            Self::MissingBaseUrl => f.write_str("openai-compat base_url is required"),
            Self::MissingApiKey => f.write_str("openai-compat api_key is required"),
            Self::InvalidBaseUrl(base_url) => {
                write!(f, "openai-compat base_url is invalid: {base_url}")
            }
            Self::InvalidTimeoutSeconds(timeout_seconds) => {
                write!(
                    f,
                    "openai-compat timeout_seconds must be greater than zero: {timeout_seconds}"
                )
            }
        }
    }
}

impl Error for OpenAICompatConfigError {}

#[cfg(test)]
mod tests {
    use super::{OpenAICompatConfig, OpenAICompatConfigError, OpenAICompatProvider, Quirks};
    use aigw_openai::{HttpTransportConfig, OpenAIAuthConfig};
    use std::collections::BTreeMap;

    fn config() -> OpenAICompatConfig {
        OpenAICompatConfig {
            name: "groq".to_owned(),
            http: HttpTransportConfig {
                base_url: "https://api.groq.com/openai/v1/".to_owned(),
                timeout_seconds: 30,
                default_headers: BTreeMap::new(),
            },
            auth: OpenAIAuthConfig {
                api_key: "test-key".to_owned(),
                organization: None,
                project: None,
            },
            quirks: Quirks {
                supports_vision: false,
                ..Quirks::default()
            },
        }
    }

    #[test]
    fn new_normalizes_trailing_slash() {
        let provider = OpenAICompatProvider::new(config()).expect("provider should be valid");
        assert_eq!(provider.base_url(), "https://api.groq.com/openai/v1");
        assert!(!provider.quirks().supports_vision);
    }

    #[test]
    fn new_rejects_invalid_base_url() {
        let mut config = config();
        config.http.base_url = "api.groq.com/openai/v1".to_owned();

        let err = OpenAICompatProvider::new(config).expect_err("provider should be invalid");
        assert_eq!(
            err,
            OpenAICompatConfigError::InvalidBaseUrl("api.groq.com/openai/v1".to_owned())
        );
    }

    #[test]
    fn new_rejects_zero_timeout() {
        let mut config = config();
        config.http.timeout_seconds = 0;

        let err = OpenAICompatProvider::new(config).expect_err("provider should be invalid");
        assert_eq!(err, OpenAICompatConfigError::InvalidTimeoutSeconds(0));
    }

    #[test]
    fn debug_redacts_api_key() {
        let provider = OpenAICompatProvider::new(config()).unwrap();
        let debug = format!("{provider:?}");
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
    fn deserialize_minimal_config() {
        let json = r#"{
            "name": "together",
            "base_url": "https://api.together.xyz/v1",
            "api_key": "tok-xxx"
        }"#;

        let config: OpenAICompatConfig = serde_json::from_str(json).unwrap();
        let provider = OpenAICompatProvider::new(config).unwrap();
        assert_eq!(provider.name(), "together");
        assert!(provider.quirks().supports_chat_completions);
        assert!(!provider.quirks().supports_responses_api);
    }

    #[test]
    fn deserialize_with_partial_quirks() {
        let json = r#"{
            "name": "vllm",
            "base_url": "http://localhost:8000/v1",
            "api_key": "none",
            "quirks": {
                "supports_vision": false,
                "supports_parallel_tool_calls": false
            }
        }"#;

        let config: OpenAICompatConfig = serde_json::from_str(json).unwrap();
        let provider = OpenAICompatProvider::new(config).unwrap();
        assert!(!provider.quirks().supports_vision);
        assert!(!provider.quirks().supports_parallel_tool_calls);
        assert!(provider.quirks().supports_streaming);
    }
}
