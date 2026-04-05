//! Event logging types for the Claude Code telemetry endpoint.
//!
//! This is a **non-standard** endpoint used by Claude Code to report telemetry
//! events to Anthropic. It is not part of the public Anthropic API.

use serde::{Deserialize, Serialize};

/// POST `/api/event_logging/batch` request body.
///
/// Events are opaque JSON values — we forward them without interpretation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventLoggingRequest {
    /// Batch of telemetry events.
    pub events: Vec<serde_json::Value>,

    /// Forward-compatible extra fields.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// POST `/api/event_logging/batch` response body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventLoggingResponse {
    /// Forward-compatible extra fields.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}
