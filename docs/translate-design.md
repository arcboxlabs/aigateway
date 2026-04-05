# Translation Layer Design

Design decisions and implementation notes for the `aigw-core` translator trait implementations.

## Core Principle: Translation Lives Inside Each Provider

The translator is **not** a standalone crate or top-level directory. Each provider owns its
translation logic as a `translate/` submodule:

```
providers/aigw-anthropic/src/
├── translate/
│   ├── mod.rs
│   ├── request.rs      # impl RequestTranslator
│   ├── response.rs     # impl ResponseTranslator
│   ├── stream.rs       # impl StreamParser
│   └── tools.rs        # tool_use/tool_result mapping (complex enough to justify its own file)
├── types/              # Anthropic native serde types (already exists)
├── client.rs           # HTTP client (already exists)
└── ...
```

**Why not a shared `translate/` crate:**
- Translation and provider are 1:1 bound — Anthropic's translator is never reused by Gemini.
- Splitting them forces cross-directory jumping when modifying a single provider.
- No actual code sharing benefit — each provider's mapping logic is unique.

## OpenAI Compat Reuses OpenAI Directly

`aigw-openai-compat` already depends on `aigw-openai` (for `HttpTransportConfig`, `OpenAIAuthConfig`).
This dependency is natural: compat providers speak the same wire format.

**No `format-openai` crate needed.** Reasons:
1. `aigw-openai`'s `wire_types/` IS the format definition — extracting it just moves files around.
2. Wire types and client have tight coupling (`OpenAIClient::chat()` returns `ChatCompletionResponse`).
3. An extra crate adds overhead without reducing actual coupling.

The compat translator wraps OpenAI's translator and applies quirks-based field stripping:

```rust
pub struct OpenAICompatRequestTranslator {
    inner: aigw_openai::translate::OpenAIRequestTranslator,
    quirks: Quirks,
}
```

## Data Flow

```
ChatRequest (aigw-core canonical)
    │
    │  RequestTranslator::translate_request()
    │  Pure data mapping — no IO, no config reads
    ▼
TranslatedRequest { url, method, headers, body: Bytes }
    │
    │  HTTP layer (outside translator's scope)
    ▼
Raw HTTP Response (StatusCode + body bytes)
    │
    ├── Non-streaming: ResponseTranslator::translate_response()
    ├── Streaming:     StreamParser::parse_event() (per SSE event)
    ▼
ChatResponse / Vec<StreamEvent> (aigw-core canonical)
```

## Translator Struct Holds Config, Functions Are Pure

The translator struct is constructed with config values (base_url, api_version, etc.)
injected at creation time. Translation functions themselves are pure: `(&self, input) → output`.

```rust
pub struct AnthropicRequestTranslator {
    base_url: String,
    api_version: String,
    default_max_tokens: u64,
}

impl RequestTranslator for AnthropicRequestTranslator {
    fn translate_request(&self, req: &ChatRequest) -> Result<TranslatedRequest, TranslateError> {
        // Uses self.base_url, self.default_max_tokens
        // Does NOT read from config module or environment
    }
}
```

**Do not**: `use super::config::Config` in translate modules.
**Do**: pass needed values via struct fields at construction time.

## Per-Provider Complexity

### OpenAI — Near Passthrough
- Canonical format IS OpenAI format → minimal translation.
- `translate/` could even be a single `translate.rs` file.
- Main work: build URL, set auth headers, serialize body.
- No `tools.rs` needed — tool format is already canonical.

### Anthropic — Most Complex
- System message extraction (from messages array → top-level `system` field)
- Role mapping: `tool` → `user` with `tool_result` content blocks
- Content block restructuring (flat content → typed blocks)
- Tool definition: unwrap `function` wrapper, rename `parameters` → `input_schema`
- Tool choice: string modes → tagged objects (`"required"` → `{ type: "any" }`)
- Tool call arguments: JSON string → JSON object (request), object → string (response)
- Consecutive tool messages → single user message with multiple tool_result blocks
- **Streaming**: stateful — tracks tool_call_index, captures id/model from message_start
- Separate `tools.rs` justified by complexity.

### Gemini — Different Paradigm
- Model in URL path, not body
- Auth via query param (`?key=`) or header (`x-goog-api-key`)
- Content always `parts: [Part]`, never plain string
- Role mapping: `assistant` → `model`, `tool` → `function`
- Tool args as JSON object (not string)
- Schema types UPPERCASE
- Streaming returns snapshots, not deltas — parser must diff consecutive events
- Separate `safety.rs` if safety_ratings mapping grows complex

### OpenAI Compat — Thin Wrapper
- Single `translate.rs` file (no directory needed)
- Wraps OpenAI translator + strips fields based on Quirks
- Validates capabilities before sending (e.g., reject streaming if unsupported)

## Testing Strategy

### Fixture-Based, No Network

Each provider's tests live at `providers/aigw-{name}/tests/` with JSON fixtures:

```
tests/
├── fixtures/
│   ├── chat_request.json              # Canonical input
│   ├── chat_request_native.json       # Expected translated output
│   ├── chat_response_native.json      # Provider's raw response
│   ├── chat_response.json             # Expected canonical output
│   ├── stream_replay.sse             # Full SSE event sequence
│   └── error_responses/
│       ├── rate_limit.json
│       └── auth_failure.json
```

### Test Types

1. **Request translation**: canonical JSON → translator → compare native body against fixture
2. **Response translation**: native JSON → translator → compare canonical output against fixture
3. **SSE replay**: feed recorded SSE lines through StreamParser, verify complete StreamEvent sequence
4. **Round-trip**: canonical → native → canonical (where applicable)
5. **Extra field preservation**: fixtures include unknown fields, verify they survive translation in `extra`
6. **Error mapping**: HTTP status + error body → ProviderError variant

### Fixture Sources

- Copy from official API documentation examples
- Capture real responses via `curl` and sanitize secrets
- Never hand-write fixtures — real API responses have subtle differences (field ordering, null vs absent)

## Implementation Order

Recommended order (each step is independently shippable):

1. **OpenAI translator** — near passthrough, establishes the pattern
2. **OpenAI compat translator** — wraps OpenAI, validates the composition model
3. **Anthropic translator** — most complex, most value
4. **Gemini translator** — can be deferred (skeleton status)

Within each provider, implement in this order:
1. Request translation (non-streaming)
2. Response translation (non-streaming)
3. Stream parser
4. Error translation
