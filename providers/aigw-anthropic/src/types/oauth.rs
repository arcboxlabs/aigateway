//! OAuth token types for the Claude Code authentication flow.
//!
//! This is a **non-standard** endpoint used by Claude Code to exchange and
//! refresh OAuth tokens. It is not part of the public Anthropic API.

use secrecy::SecretString;
use serde::{Deserialize, Serialize};

/// POST `/v1/oauth/token` request body.
#[derive(Debug, Clone, Serialize)]
pub struct OAuthTokenRequest {
    /// Grant type — `"authorization_code"` or `"refresh_token"`.
    pub grant_type: String,
    /// OAuth client ID.
    pub client_id: String,

    /// Authorization code (for `authorization_code` grant).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Redirect URI (for `authorization_code` grant).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_uri: Option<String>,
    /// PKCE code verifier (for `authorization_code` grant).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_verifier: Option<String>,

    /// Refresh token (for `refresh_token` grant).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// Requested scopes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,

    /// Forward-compatible extra fields.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// POST `/v1/oauth/token` response body.
///
/// `access_token` and `refresh_token` are [`SecretString`] to prevent
/// accidental exposure in logs or Debug output.
#[derive(Clone, Deserialize)]
pub struct OAuthTokenResponse {
    /// Bearer access token (e.g. `"sk-ant-oat01-..."`).
    #[serde(deserialize_with = "deserialize_secret")]
    pub access_token: SecretString,
    /// Refresh token for obtaining new access tokens.
    #[serde(default, deserialize_with = "deserialize_option_secret")]
    pub refresh_token: Option<SecretString>,
    /// Token lifetime in seconds.
    pub expires_in: u64,
    /// Token type — typically `"bearer"`.
    pub token_type: String,

    /// Forward-compatible extra fields (organization, account, etc.).
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl std::fmt::Debug for OAuthTokenResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuthTokenResponse")
            .field("access_token", &"[REDACTED]")
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "[REDACTED]"),
            )
            .field("expires_in", &self.expires_in)
            .field("token_type", &self.token_type)
            .finish()
    }
}

fn deserialize_secret<'de, D>(deserializer: D) -> Result<SecretString, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(SecretString::from(s))
}

fn deserialize_option_secret<'de, D>(deserializer: D) -> Result<Option<SecretString>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    Ok(s.map(SecretString::from))
}
