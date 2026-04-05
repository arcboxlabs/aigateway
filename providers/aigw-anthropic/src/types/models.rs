//! Model listing types for the Anthropic API.
//!
//! See <https://docs.anthropic.com/en/api/models>

use serde::Deserialize;

/// GET `/v1/models` response body.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelListResponse {
    /// List of available models.
    pub data: Vec<Model>,
    /// Whether there are more models to fetch.
    pub has_more: bool,
    /// ID of the first model in this page.
    #[serde(default)]
    pub first_id: Option<String>,
    /// ID of the last model in this page.
    #[serde(default)]
    pub last_id: Option<String>,
}

/// A single model entry.
#[derive(Debug, Clone, Deserialize)]
pub struct Model {
    /// Model identifier (e.g. `"claude-sonnet-4-20250514"`).
    pub id: String,

    /// Forward-compatible extra fields.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}
