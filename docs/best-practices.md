# Best Practices for LLM Provider Implementation in Rust

Derived from analysis of rig-core, misanthropic, async-openai, and LiteLLM.

## 1. Type Modeling

### String-or-Array Content Duality

Both OpenAI and Anthropic allow `content` to be either a plain string or an array of typed blocks. Model this with `#[serde(untagged)]`:

```rust
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}
```

This deserializes `"hello"` as `Text` and `[{ "type": "text", "text": "hello" }]` as `Parts`.

### Content Blocks: Internally Tagged

Use `#[serde(tag = "type")]` for content part enums to match the wire format:

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}
```

### Forward Compatibility

Add an `Unknown` catch-all variant to streaming event enums:

```rust
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamingEvent {
    MessageStart { ... },
    ContentBlockDelta { ... },
    // ...
    #[serde(other)]
    Unknown,
}
```

This prevents deserialization failures when the provider adds new event types.

### Passthrough Fields

Use `#[serde(flatten)]` for provider-specific parameters that the gateway shouldn't interpret:

```rust
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    // ... known fields ...
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}
```

This is cleaner than `additional_params: Option<Value>` (used by rig) because it naturally merges into the JSON without special handling.

## 2. Provider Abstraction

### Keep the Trait Thin

For a gateway, the Provider trait should be a **protocol translation layer**, not an application framework:

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    fn capabilities(&self) -> &ProviderCapabilities;

    async fn chat_completion(
        &self, req: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, ProviderError>;

    async fn chat_completion_stream(
        &self, req: ChatCompletionRequest,
    ) -> Result<BoxStream<'static, Result<ChatCompletionChunk, ProviderError>>, ProviderError>;
}
```

Unlike rig's `CompletionModel` (which uses associated types for provider-specific responses), a gateway should always return the unified type. The translation happens inside each provider implementation.

### Provider-Specific Types with TryFrom/Into

Each provider crate defines its own native request/response types that exactly mirror the provider's API. Conversion between unified and native types uses `From`/`TryFrom`:

```rust
// In aigw-anthropic:
impl TryFrom<ChatCompletionRequest> for AnthropicRequest { ... }
impl TryFrom<AnthropicResponse> for ChatCompletionResponse { ... }
```

This keeps translation logic isolated and testable.

### Capabilities Declaration

Use a struct (not bitflags) for readability:

```rust
pub struct ProviderCapabilities {
    pub streaming: bool,
    pub tool_calling: bool,
    pub vision: bool,
    pub json_mode: bool,
}
```

The gateway checks capabilities before routing requests. If a request uses `tools` but the provider doesn't support `tool_calling`, reject early with a clear error.

## 3. Streaming Implementation

### Use `async_stream::stream!`

The `stream!` macro (used by rig) produces cleaner code than `tokio::spawn` + `mpsc` channel (used by async-openai):

```rust
async fn stream(req: Request) -> Result<BoxStream<'static, Result<Chunk, Error>>, Error> {
    let response = self.client.post(...).send().await?;
    let event_stream = parse_sse(response.bytes_stream());

    Ok(Box::pin(async_stream::stream! {
        for await event in event_stream {
            match translate_event(event) {
                Some(chunk) => yield Ok(chunk),
                None => continue,  // skip ping, unknown events
            }
        }
    }))
}
```

### State Machine for Streaming Translation

Anthropic's block-level events need to be translated to OpenAI's choice-level deltas. Maintain mutable state:

```rust
struct StreamState {
    msg_id: String,
    model: String,
    created: u64,
    tool_call_index: i32,  // incremented on each tool_use content_block_start
}
```

Key state transitions:
- `message_start` → capture `id`, `model`; emit role chunk
- `content_block_start(tool_use)` → increment `tool_call_index`; emit tool call header
- `content_block_delta(text_delta)` → emit content delta
- `content_block_delta(input_json_delta)` → emit tool call arguments delta
- `message_delta` → emit finish_reason chunk
- `message_stop` → emit `[DONE]`
- `content_block_stop` → no output (state cleanup only)
- `ping` → ignore

### SSE Parsing

Use the `eventsource-stream` crate for parsing SSE from a byte stream. It handles `event:` and `data:` line parsing, multi-line data, and retry fields.

## 4. Error Handling

### Typed Provider Errors

Map API-level errors to typed enums (pattern from misanthropic):

```rust
pub enum ProviderError {
    /// HTTP transport error
    Http(reqwest::Error),
    /// JSON serialization/deserialization error
    Serialization(serde_json::Error),
    /// Provider returned an API error
    Api { status: u16, error_type: String, message: String },
    /// Provider doesn't support requested capability
    UnsupportedCapability(String),
}
```

The `Api` variant carries the provider's error type and message, enabling the gateway to return meaningful error responses.

### Anthropic-Specific Error Format

```json
{
  "type": "error",
  "error": {
    "type": "invalid_request_error",
    "message": "max_tokens: field required"
  }
}
```

Map to appropriate HTTP status codes:
- `invalid_request_error` → 400
- `authentication_error` → 401
- `permission_error` → 403
- `not_found_error` → 404
- `rate_limit_error` → 429
- `overloaded_error` → 529

## 5. Key Edge Cases

### Message Merging (Anthropic)

Anthropic requires strict user/assistant alternation. When translating from OpenAI format:
- Consecutive `role: "tool"` messages → merge into single `role: "user"` with multiple `tool_result` blocks
- Consecutive `role: "user"` messages → merge contents into one message
- Multiple `role: "system"` messages → concatenate into single `system` field

### Tool Call ID Generation (Gemini)

Gemini doesn't provide tool call IDs. Generate deterministic UUIDs (e.g. `uuid::Uuid::new_v4()`) and maintain a mapping for tool result routing.

### max_tokens Default

Anthropic requires `max_tokens`. When absent from the unified request:
- Use model-specific defaults (e.g. 4096 for most models, 8192 for Opus)
- Or require it in the gateway config

### Content-Only vs Tool-Only Responses

When translating Anthropic → OpenAI:
- If only `text` blocks: `content = "concatenated text"`, `tool_calls = null`
- If only `tool_use` blocks: `content = null`, `tool_calls = [...]`
- If mixed: `content = "text"`, `tool_calls = [...]`

### Streaming Usage

- Anthropic: `usage` appears in `message_start` (input_tokens) and `message_delta` (output_tokens, cumulative)
- OpenAI: `usage` appears in the final chunk only
- Emit usage in the last chunk before `[DONE]`

## 6. Provider Crate Architecture

### The OpenAI-Compatible Ecosystem Problem

In practice, dozens of providers (vLLM, Together AI, Groq, Fireworks, Perplexity, LM Studio, etc.) clone the OpenAI API shape but differ in:
- `base_url`
- Supported endpoint subset (most lack the Responses API)
- Feature coverage (`tools`, `tool_choice`, `vision`, `parallel_tool_calls`)
- Silent field ignoring (accept a field but do nothing with it)

Writing a separate crate per provider is wasteful. Instead, the project uses a **four-crate strategy**:

### Crate Responsibilities

| Crate | Scope | Translation |
|-------|-------|-------------|
| `aigw-openai` | OpenAI official API only | Near-passthrough; maximum protocol fidelity |
| `aigw-openai-compat` | All OpenAI-shaped third-party APIs | Configurable passthrough with `Quirks` capability matrix |
| `aigw-anthropic` | Anthropic Messages API | Full protocol translation |
| `aigw-gemini` | Google Gemini API | Full protocol translation |

### Why Separate `openai` and `openai-compat`

- `aigw-openai` targets **protocol fidelity** with OpenAI's official API — it can assume all features exist, use OpenAI-specific headers (`OpenAI-Organization`, `OpenAI-Project`), and track OpenAI-specific extensions (Responses API, realtime, etc.)
- `aigw-openai-compat` targets the **lowest common denominator** of the OpenAI-shaped ecosystem. It makes no assumptions about feature support; everything is declared via `Quirks`.

Merging them would either limit OpenAI support (can't assume features) or break compat providers (assume features that don't exist).

### Quirks-Based Capability Declaration

```rust
pub struct Quirks {
    pub supports_responses_api: bool,
    pub supports_chat_completions: bool,
    pub supports_embeddings: bool,
    pub supports_streaming: bool,
    pub supports_tool_choice: bool,
    pub supports_parallel_tool_calls: bool,
    pub supports_vision: bool,
}

impl Default for Quirks {
    fn default() -> Self {
        Self {
            supports_responses_api: false,     // most compat providers lack this
            supports_chat_completions: true,
            supports_embeddings: true,
            supports_streaming: true,
            supports_tool_choice: true,
            supports_parallel_tool_calls: true,
            supports_vision: true,
        }
    }
}
```

Defaults are optimistic for chat but pessimistic for newer features (Responses API). Provider-specific presets can override:

```rust
let groq = OpenAICompatProvider::new(OpenAICompatConfig {
    name: "groq".into(),
    base_url: "https://api.groq.com/openai/v1".into(),
    api_key: env::var("GROQ_API_KEY")?,
    quirks: Quirks {
        supports_vision: false,
        ..Quirks::default()
    },
    ..Default::default()
});
```

### Shared Transport Layer

`aigw-openai` and `aigw-openai-compat` should share an HTTP/SSE transport layer since the wire format is identical. Extract common logic (SSE parsing, request building, response deserialization) into shared utilities rather than duplicating between the two crates.

### Adding New Providers: Decision Tree

```
Does the provider use OpenAI-compatible API format?
├── Yes → Use `aigw-openai-compat` with appropriate Quirks
│         (vLLM, Groq, Together, Fireworks, Perplexity, LM Studio, ...)
└── No  → Does it need full protocol translation?
          ├── Yes → New crate (like `aigw-anthropic`, `aigw-gemini`)
          └── Unlikely — most providers either clone OpenAI or have a distinct API
```
