//! Token counting types for the Anthropic Messages API.
//!
//! See <https://docs.anthropic.com/en/api/counting-tokens>

use bon::Builder;
use serde::{Deserialize, Serialize};

use super::messages::{Message, SystemPrompt, ThinkingConfig, Tool};

/// POST `/v1/messages/count_tokens` request body.
#[derive(Debug, Clone, Builder, Serialize)]
#[builder(on(String, into))]
pub struct CountTokensRequest {
    /// Model identifier.
    pub model: String,
    /// Input messages.
    pub messages: Vec<Message>,

    /// System prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<SystemPrompt>,
    /// Available tools (counted toward input tokens).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    /// Extended thinking configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,

    /// Forward-compatible extra fields.
    #[serde(flatten)]
    #[builder(default)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// POST `/v1/messages/count_tokens` response body.
#[derive(Debug, Clone, Deserialize)]
pub struct CountTokensResponse {
    /// The estimated number of input tokens.
    pub input_tokens: u64,

    /// Forward-compatible extra fields.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}
