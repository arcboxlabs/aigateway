//! Google Gemini API native types.
//!
//! These types map 1:1 to the Gemini wire format.
//! See <https://ai.google.dev/api/generate-content>
//!
//! # Naming Convention
//!
//! The Gemini REST API uses **camelCase** for all fields (e.g. `generationConfig`,
//! `maxOutputTokens`, `functionDeclarations`).
//!
//! Enums use **SCREAMING_SNAKE_CASE** (e.g. `STOP`, `MAX_TOKENS`,
//! `HARM_CATEGORY_HATE_SPEECH`).

use bon::Builder;
use serde::{Deserialize, Serialize};

// ─── Request ────────────────────────────────────────────────────────────────

/// `generateContent` request body.
///
/// The [`model`](Self::model) field is placed in the URL path by the client,
/// not serialized in the JSON body.
///
/// # Example
///
/// ```
/// use aigw_gemini::types::*;
///
/// let req = GenerateContentRequest::builder()
///     .model("gemini-2.5-flash")
///     .contents(vec![Content {
///         role: Some(Role::User),
///         parts: vec![Part::text("Hello")],
///     }])
///     .generation_config(GenerationConfig {
///         temperature: Some(0.7),
///         max_output_tokens: Some(1024),
///         ..Default::default()
///     })
///     .build();
/// ```
#[derive(Debug, Clone, Builder, Serialize)]
#[serde(rename_all = "camelCase")]
#[builder(on(String, into))]
pub struct GenerateContentRequest {
    /// Model identifier (e.g. `"gemini-2.5-flash"`).
    /// Placed in the URL path, not serialized in the body.
    #[serde(skip)]
    pub model: String,

    /// Input contents (conversation history).
    pub contents: Vec<Content>,

    /// Available tools (function declarations, Google Search, code execution, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    /// Tool selection configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<ToolConfig>,
    /// Safety settings to filter harmful content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_settings: Option<Vec<SafetySetting>>,
    /// System instruction — a [`Content`] with no role, containing the system prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<Content>,
    /// Generation parameters (temperature, max tokens, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
    /// Cached content resource name (e.g. `"cachedContents/{id}"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_content: Option<String>,

    /// Provider-specific fields that we don't interpret.
    #[serde(flatten)]
    #[builder(default)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

// ─── Content & Parts ────────────────────────────────────────────────────────

/// A conversation turn containing a role and content parts.
///
/// For `systemInstruction`, the role is typically omitted (`None`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    /// Conversation role — `"user"`, `"model"`, or `"function"`.
    /// Omitted for system instructions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
    /// Content parts. Gemini always uses an array of parts, never a plain string.
    pub parts: Vec<Part>,
}

/// Conversation role.
///
/// Gemini uses `"model"` instead of `"assistant"`, and `"function"` for
/// tool result messages (older style; newer models accept `"user"` with
/// `functionResponse` parts).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// User message.
    User,
    /// Model (assistant) message.
    Model,
    /// Function response message (legacy tool result role).
    Function,
    /// Forward-compatible catch-all for unknown roles.
    #[serde(untagged)]
    Other(String),
}

/// A content part within a [`Content`] turn.
///
/// Gemini parts use a protobuf **oneof** pattern: each part has exactly one
/// "data" field present (`text`, `inline_data`, `function_call`, etc.), plus
/// optional metadata fields (`thought`, `thought_signature`) that can co-exist.
///
/// This is modeled as a flat struct with all-optional fields for maximum
/// forward compatibility. Unknown fields flow into [`extra`](Self::extra).
///
/// Use the convenience constructors ([`Part::text`], [`Part::inline_data`], etc.)
/// for ergonomic creation.
///
/// ```
/// # use aigw_gemini::types::*;
/// let text_part = Part::text("Hello, Gemini!");
/// assert!(text_part.text.is_some());
///
/// let thought_part = Part {
///     text: Some("Let me think...".into()),
///     thought: Some(true),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    // ── Content fields (exactly one should be present) ──
    /// Plain text content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Base64-encoded inline binary data (images, audio, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_data: Option<Blob>,
    /// Reference to a file (Cloud Storage URI, HTTPS URL, or Files API URI).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_data: Option<FileData>,
    /// Function call from the model (tool invocation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCall>,
    /// Function response from the user (tool result).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_response: Option<FunctionResponse>,
    /// Code to execute (code execution tool).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable_code: Option<ExecutableCode>,
    /// Result of code execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_execution_result: Option<CodeExecutionResult>,

    // ── Metadata (can co-exist with content fields) ──
    /// Whether this part is a thinking/reasoning step.
    /// Present in Gemini 2.5+ when thinking is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought: Option<bool>,
    /// Opaque encrypted signature for thinking parts.
    /// **Must** be preserved and returned as-is in subsequent conversation turns.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought_signature: Option<String>,

    /// Forward-compatible catch-all for unknown fields.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Part {
    /// Create a text part.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            ..Default::default()
        }
    }

    /// Create an inline data part (base64-encoded binary).
    pub fn inline_data(blob: Blob) -> Self {
        Self {
            inline_data: Some(blob),
            ..Default::default()
        }
    }

    /// Create a file data part (URI reference).
    pub fn file_data(data: FileData) -> Self {
        Self {
            file_data: Some(data),
            ..Default::default()
        }
    }

    /// Create a function call part.
    pub fn function_call(call: FunctionCall) -> Self {
        Self {
            function_call: Some(call),
            ..Default::default()
        }
    }

    /// Create a function response part.
    pub fn function_response(response: FunctionResponse) -> Self {
        Self {
            function_response: Some(response),
            ..Default::default()
        }
    }
}

/// Base64-encoded binary data with MIME type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Blob {
    /// MIME type (e.g. `"image/png"`, `"audio/mp3"`).
    pub mime_type: String,
    /// Base64-encoded bytes.
    pub data: String,
}

/// File reference by URI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileData {
    /// MIME type of the referenced file.
    pub mime_type: String,
    /// Cloud Storage URI (`gs://`), HTTPS URL, YouTube URL, or Files API URI.
    pub file_uri: String,
}

/// Function call from the model — a request to invoke a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Function name.
    pub name: String,
    /// Function arguments as a JSON object.
    pub args: serde_json::Value,
    /// Unique call ID. Always present in Gemini 3+ models; may be absent in older models.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// Function response — the user's result for a function call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionResponse {
    /// Function name (must match the [`FunctionCall::name`]).
    pub name: String,
    /// Function output as a JSON object.
    pub response: serde_json::Value,
    /// Call ID (must match [`FunctionCall::id`] if present).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// Code to execute via the code execution tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutableCode {
    /// Programming language.
    pub language: Language,
    /// Source code.
    pub code: String,
}

/// Programming language for code execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Language {
    /// Python.
    Python,
    /// Forward-compatible catch-all.
    #[serde(untagged)]
    Other(String),
}

/// Result of code execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExecutionResult {
    /// Execution outcome.
    pub outcome: CodeExecutionOutcome,
    /// stdout on success, stderr on failure. May be absent if no output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

/// Outcome of code execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CodeExecutionOutcome {
    /// Execution succeeded.
    #[serde(rename = "OUTCOME_OK")]
    Ok,
    /// Execution failed.
    #[serde(rename = "OUTCOME_FAILED")]
    Failed,
    /// Execution timed out.
    #[serde(rename = "OUTCOME_DEADLINE_EXCEEDED")]
    DeadlineExceeded,
    /// Forward-compatible catch-all.
    #[serde(untagged)]
    Unknown(String),
}

// ─── Tools ──────────────────────────────────────────────────────────────────

/// Tool definition.
///
/// Each tool object can contain one or more tool types. Function declarations
/// are the most common; the others enable built-in capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    /// Function tool declarations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_declarations: Option<Vec<FunctionDeclaration>>,
    /// Enable Google Search grounding (pass empty object `{}`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_search: Option<serde_json::Value>,
    /// Enable code execution (pass empty object `{}`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_execution: Option<serde_json::Value>,
    /// Enable URL context fetching (pass empty object `{}`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_context: Option<serde_json::Value>,

    /// Forward-compatible catch-all for unknown tool types.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Function declaration — describes a callable tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDeclaration {
    /// Function name.
    pub name: String,
    /// Human-readable description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// JSON Schema for the function's input parameters.
    /// Uses an OpenAPI 3.0 subset with **UPPERCASE** type names
    /// (e.g. `"STRING"`, `"OBJECT"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
}

/// Tool selection configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolConfig {
    /// Function calling configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_calling_config: Option<FunctionCallingConfig>,
}

/// Function calling configuration within [`ToolConfig`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionCallingConfig {
    /// Calling mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<FunctionCallingMode>,
    /// Restrict to these function names (only when mode is `Any`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_function_names: Option<Vec<String>>,
}

/// Function calling mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FunctionCallingMode {
    /// Model decides whether to call functions.
    Auto,
    /// Model must call a function every turn.
    Any,
    /// Function calling is disabled.
    None,
    /// Function calls are schema-validated (preview).
    Validated,
    /// Forward-compatible catch-all.
    #[serde(untagged)]
    Unknown(String),
}

// ─── Safety ─────────────────────────────────────────────────────────────────

/// Safety setting — configures content filtering for a harm category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetySetting {
    /// The harm category to configure.
    pub category: HarmCategory,
    /// The blocking threshold.
    pub threshold: HarmBlockThreshold,
}

/// Harm category for safety filtering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HarmCategory {
    /// Hate speech.
    #[serde(rename = "HARM_CATEGORY_HATE_SPEECH")]
    HateSpeech,
    /// Sexually explicit content.
    #[serde(rename = "HARM_CATEGORY_SEXUALLY_EXPLICIT")]
    SexuallyExplicit,
    /// Dangerous content.
    #[serde(rename = "HARM_CATEGORY_DANGEROUS_CONTENT")]
    DangerousContent,
    /// Harassment.
    #[serde(rename = "HARM_CATEGORY_HARASSMENT")]
    Harassment,
    /// Civic integrity.
    #[serde(rename = "HARM_CATEGORY_CIVIC_INTEGRITY")]
    CivicIntegrity,
    /// Forward-compatible catch-all.
    #[serde(untagged)]
    Unknown(String),
}

/// Harm blocking threshold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HarmBlockThreshold {
    /// Block when low probability or above.
    BlockLowAndAbove,
    /// Block when medium probability or above.
    BlockMediumAndAbove,
    /// Block only high probability.
    BlockOnlyHigh,
    /// Don't block (still reports ratings).
    BlockNone,
    /// Completely disable safety filtering.
    Off,
    /// Forward-compatible catch-all.
    #[serde(untagged)]
    Unknown(String),
}

// ─── Generation Config ──────────────────────────────────────────────────────

/// Generation parameters.
///
/// All fields are optional — only include the ones you want to override.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationConfig {
    /// Sampling temperature (0.0–2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Nucleus sampling parameter (0.0–1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    /// Top-K sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    /// Number of candidates to generate (1–8).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_count: Option<u32>,
    /// Maximum number of output tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u64>,
    /// Custom stop sequences (max 5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Presence penalty (−2.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,
    /// Frequency penalty (−2.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,
    /// Response MIME type (`"text/plain"`, `"application/json"`, `"text/x.enum"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_mime_type: Option<String>,
    /// Response schema for structured output (OpenAPI subset with UPPERCASE types).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_schema: Option<serde_json::Value>,
    /// Response modalities (`["TEXT"]`, `["IMAGE"]`, `["AUDIO"]`, or combinations).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_modalities: Option<Vec<String>>,
    /// Seed for reproducibility.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    /// Whether to return log probabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_logprobs: Option<bool>,
    /// Number of top log probabilities to return (1–20).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<u32>,
    /// Thinking/reasoning configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_config: Option<ThinkingConfig>,

    /// Forward-compatible catch-all for fields like `speechConfig`, `routingConfig`, etc.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Extended thinking configuration.
///
/// Gemini 2.5 uses `thinking_budget`; Gemini 3 uses `thinking_level`.
/// Include whichever is appropriate for the target model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingConfig {
    /// Token budget for thinking (Gemini 2.5).
    /// `-1` = dynamic, `0` = disabled, or `128`–`32768`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_budget: Option<i64>,
    /// Thinking level preset (Gemini 3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_level: Option<ThinkingLevel>,
    /// Whether to include thought summaries in the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_thoughts: Option<bool>,
}

/// Thinking level preset for Gemini 3 models.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ThinkingLevel {
    /// Minimal thinking.
    Minimal,
    /// Low thinking.
    Low,
    /// Medium thinking.
    Medium,
    /// High thinking (default for most models).
    High,
    /// Forward-compatible catch-all.
    #[serde(untagged)]
    Other(String),
}

// ─── Response ───────────────────────────────────────────────────────────────

/// `generateContent` response body.
///
/// Also used as each streaming chunk — Gemini sends a complete response
/// object per SSE event (not deltas).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentResponse {
    /// Generated candidates (usually one).
    #[serde(default)]
    pub candidates: Vec<Candidate>,
    /// Feedback on the prompt (present when the prompt is blocked).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_feedback: Option<PromptFeedback>,
    /// Token usage statistics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_metadata: Option<UsageMetadata>,
    /// Model version string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_version: Option<String>,
    /// Unique response ID (same across streaming chunks).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_id: Option<String>,

    /// Forward-compatible catch-all for unknown fields.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// A generated candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    /// Generated content.
    pub content: Option<Content>,
    /// Reason the model stopped generating.
    pub finish_reason: Option<FinishReason>,
    /// Safety ratings for this candidate.
    #[serde(default)]
    pub safety_ratings: Vec<SafetyRating>,
    /// Citation information.
    pub citation_metadata: Option<CitationMetadata>,
    /// Google Search grounding metadata.
    pub grounding_metadata: Option<GroundingMetadata>,
    /// Candidate index (for multi-candidate responses).
    pub index: Option<u32>,
    /// Average log probability across generated tokens.
    pub avg_logprobs: Option<f64>,

    /// Forward-compatible catch-all (e.g. `logprobsResult`, `urlContextMetadata`).
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Reason the model stopped generating.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FinishReason {
    /// Model finished naturally.
    Stop,
    /// Hit the max output token limit.
    MaxTokens,
    /// Blocked by safety filters.
    Safety,
    /// Blocked due to recitation (copyright).
    Recitation,
    /// Blocked due to unsupported language.
    Language,
    /// Other unspecified reason.
    Other,
    /// Blocked by blocklist.
    Blocklist,
    /// Blocked due to prohibited content.
    ProhibitedContent,
    /// Blocked due to Sensitive PII.
    Spii,
    /// Function call was malformed.
    MalformedFunctionCall,
    /// Blocked due to image safety.
    ImageSafety,
    /// No image was generated.
    NoImage,
    /// Forward-compatible catch-all.
    #[serde(untagged)]
    Unknown(String),
}

/// Token usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetadata {
    /// Number of input (prompt) tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_token_count: Option<u64>,
    /// Number of output (candidates) tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidates_token_count: Option<u64>,
    /// Total tokens (prompt + candidates).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_token_count: Option<u64>,
    /// Tokens from cached content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_content_token_count: Option<u64>,
    /// Tokens used for thinking/reasoning.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thoughts_token_count: Option<u64>,
}

// ─── Safety Ratings ─────────────────────────────────────────────────────────

/// Safety rating for a generated candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyRating {
    /// The harm category.
    pub category: HarmCategory,
    /// Probability of harm.
    pub probability: HarmProbability,
    /// Whether this rating caused the content to be blocked.
    #[serde(default)]
    pub blocked: bool,
}

/// Probability level for a safety rating.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HarmProbability {
    /// Negligible probability.
    Negligible,
    /// Low probability.
    Low,
    /// Medium probability.
    Medium,
    /// High probability.
    High,
    /// Forward-compatible catch-all.
    #[serde(untagged)]
    Unknown(String),
}

// ─── Prompt Feedback ────────────────────────────────────────────────────────

/// Feedback on the prompt, present when the prompt is blocked.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptFeedback {
    /// Reason the prompt was blocked.
    pub block_reason: Option<BlockReason>,
    /// Safety ratings for the prompt.
    #[serde(default)]
    pub safety_ratings: Vec<SafetyRating>,
}

/// Reason a prompt was blocked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BlockReason {
    /// Blocked by safety filters.
    Safety,
    /// Other reason.
    Other,
    /// Blocked by blocklist.
    Blocklist,
    /// Prohibited content.
    ProhibitedContent,
    /// Image safety violation.
    ImageSafety,
    /// Forward-compatible catch-all.
    #[serde(untagged)]
    Unknown(String),
}

// ─── Citations ──────────────────────────────────────────────────────────────

/// Citation metadata for a candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationMetadata {
    /// List of citations.
    #[serde(default)]
    pub citation_sources: Vec<Citation>,
}

/// A single citation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Citation {
    /// Start index in the generated text.
    pub start_index: Option<u64>,
    /// End index in the generated text.
    pub end_index: Option<u64>,
    /// Source URI.
    pub uri: Option<String>,
    /// Source title.
    pub title: Option<String>,
    /// License information.
    pub license: Option<String>,
}

// ─── Grounding ──────────────────────────────────────────────────────────────

/// Google Search grounding metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroundingMetadata {
    /// Search queries performed.
    #[serde(default)]
    pub web_search_queries: Vec<String>,
    /// Search entry point with rendered HTML widget.
    pub search_entry_point: Option<SearchEntryPoint>,
    /// Source chunks used for grounding.
    #[serde(default)]
    pub grounding_chunks: Vec<GroundingChunk>,
    /// Grounding support segments.
    #[serde(default)]
    pub grounding_supports: Vec<GroundingSupport>,

    /// Forward-compatible catch-all.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Search entry point for Google Search grounding.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchEntryPoint {
    /// Rendered HTML/CSS content for the search widget.
    pub rendered_content: Option<String>,
}

/// A grounding source chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundingChunk {
    /// Web source.
    pub web: Option<GroundingChunkWeb>,
}

/// Web source within a grounding chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundingChunkWeb {
    /// Source URI.
    pub uri: Option<String>,
    /// Source title.
    pub title: Option<String>,
}

/// Grounding support — links a text segment to source chunks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroundingSupport {
    /// The text segment that is grounded.
    pub segment: Option<TextSegment>,
    /// Indices into [`GroundingMetadata::grounding_chunks`].
    #[serde(default)]
    pub grounding_chunk_indices: Vec<u32>,
}

/// A segment of generated text.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextSegment {
    /// Start character index.
    pub start_index: Option<u64>,
    /// End character index.
    pub end_index: Option<u64>,
    /// The text content.
    pub text: Option<String>,
}

// ─── API Error ──────────────────────────────────────────────────────────────

/// Google API error body.
///
/// ```json
/// { "error": { "code": 400, "message": "...", "status": "INVALID_ARGUMENT" } }
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct GoogleApiError {
    /// HTTP status code.
    pub code: u16,
    /// Human-readable error message.
    pub message: String,
    /// gRPC-style status string (e.g. `"INVALID_ARGUMENT"`, `"RESOURCE_EXHAUSTED"`).
    #[serde(default)]
    pub status: String,
    /// Optional error details (field violations, etc.).
    #[serde(default)]
    pub details: Vec<serde_json::Value>,
}

/// Top-level error response wrapper.
#[derive(Debug, Clone, Deserialize)]
pub struct GoogleErrorResponse {
    /// The nested error object.
    pub error: GoogleApiError,
}
