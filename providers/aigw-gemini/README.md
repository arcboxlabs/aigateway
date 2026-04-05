# aigw-gemini

Typed Rust client for the [Google Gemini API](https://ai.google.dev/api/generate-content) (`generateContent` / `streamGenerateContent`).

## Quick Start

```rust
use aigw_gemini::{Client, ClientConfig, GenerateContentRequest, Content, Part, Role};

let client = Client::new(
    ClientConfig::builder().api_key("AIza...").build()
)?;

let req = GenerateContentRequest::builder()
    .model("gemini-2.5-flash")
    .contents(vec![Content {
        role: Some(Role::User),
        parts: vec![Part::text("Hello, Gemini!")],
    }])
    .build();

let response = client.generate_content(&req).await?;
```

## Gemini API Characteristics

Gemini differs from OpenAI and Anthropic in several important ways:

| Aspect | Gemini | OpenAI | Anthropic |
|--------|--------|--------|-----------|
| Model location | URL path | Request body | Request body |
| Auth header | `x-goog-api-key` | `Authorization: Bearer` | `x-api-key` |
| Assistant role | `"model"` | `"assistant"` | `"assistant"` |
| Content structure | Always `parts: [Part]` | String or content array | String or content array |
| Tool call args | JSON object | JSON string | JSON object |
| Tool result role | `"function"` / `"user"` | `"tool"` | `"user"` with `tool_result` block |
| Tool call IDs | Present in Gemini 3+, absent in older | Always present | Always present |
| Schema types | UPPERCASE (`"STRING"`) | lowercase (`"string"`) | lowercase (`"string"`) |
| Finish reasons | `SCREAMING_SNAKE_CASE` | `snake_case` | `snake_case` |
| Streaming | Each chunk = full response | Delta-based chunks | Named SSE events with deltas |
| Stream termination | Connection closes | `data: [DONE]` | `message_stop` event |
| Field naming | camelCase (mostly) | snake_case | snake_case |

## Design Decisions

### Part as Flat Struct (not enum)

Gemini `Part` is a protobuf **oneof** — each part has exactly one "data" field present (`text`, `inlineData`, `functionCall`, etc.), plus optional metadata (`thought`, `thoughtSignature`) that can co-exist with the data field.

We model this as a flat struct with all-`Option` fields rather than a Rust enum because:

1. **Metadata co-existence** — `thought: true` and `thoughtSignature` can appear alongside `text`, which an enum can't represent without duplicating these fields in every variant.
2. **Forward compatibility** — new part types from Google automatically flow into `#[serde(flatten)] extra` without breaking deserialization.
3. **Protobuf idiom** — flat optional fields is the standard JSON representation of protobuf `oneof`.

Convenience constructors (`Part::text()`, `Part::function_call()`, etc.) provide ergonomic creation.

### Model in URL, Not Body

Unlike OpenAI and Anthropic where the model is a JSON body field, Gemini puts the model in the URL path:

```
POST /v1beta/models/{model}:generateContent
POST /v1beta/models/{model}:streamGenerateContent?alt=sse
```

`GenerateContentRequest.model` is marked `#[serde(skip)]` so it's available for URL construction but excluded from the serialized body.

### Serde Naming Strategy

The Gemini REST API uses **camelCase** for all JSON fields (`generationConfig`,
`maxOutputTokens`, `functionDeclarations`, `googleSearch`). Enums use
**SCREAMING_SNAKE_CASE** (`STOP`, `MAX_TOKENS`, `HARM_CATEGORY_HATE_SPEECH`).

All structs use `#[serde(rename_all = "camelCase")]`. Enum variants with prefixes
(e.g. `HARM_CATEGORY_*`) use per-variant `#[serde(rename)]`.

### Streaming is Simpler Than Anthropic

Gemini streaming sends a complete `GenerateContentResponse` per SSE event (not deltas). No named SSE event types, no state machine — just deserialize each `data:` line as the response type.

## Supported Features

- **Text generation** — single and multi-turn conversations
- **Function calling** — `functionDeclarations` in tools, `functionCall`/`functionResponse` in parts
- **Google Search grounding** — `google_search` tool
- **Code execution** — `code_execution` tool with `executableCode`/`codeExecutionResult` parts
- **URL context** — `url_context` tool
- **Vision** — `inlineData` (base64) and `fileData` (URI) parts
- **Extended thinking** — `thinkingConfig` in generation config, `thought`/`thoughtSignature` in parts
- **Safety settings** — per-category harm filtering
- **Structured output** — `responseMimeType` + `responseSchema`
- **Streaming** — SSE via `streamGenerateContent?alt=sse`
- **Prompt caching** — `cachedContent` field

## Wire Format Notes

### thoughtSignature Must Be Preserved

Gemini 2.5+ (with function calling) and Gemini 3+ (always) return `thoughtSignature` fields on thinking parts. These are opaque encrypted strings that **must be returned exactly as-is** in subsequent conversation turns. The model validates them for context continuity.

### No Tool Call IDs in Older Models

Unlike OpenAI and Anthropic which always provide tool call IDs, Gemini only provides `FunctionCall.id` in Gemini 3+ models. For older models, the gateway translation layer must generate synthetic IDs.

### Schema Types Are UPPERCASE

Gemini function parameter schemas use an OpenAPI 3.0 subset with **UPPERCASE** type names (`"STRING"`, `"NUMBER"`, `"OBJECT"`), not the standard lowercase (`"string"`, `"number"`, `"object"`). The gateway translation layer must handle this conversion.

## Official References

- API Reference: https://ai.google.dev/api/generate-content
- Function Calling: https://ai.google.dev/gemini-api/docs/function-calling
- Thinking: https://ai.google.dev/gemini-api/docs/thinking
- Code Execution: https://ai.google.dev/gemini-api/docs/code-execution
- Google Search Grounding: https://ai.google.dev/gemini-api/docs/google-search
- URL Context: https://ai.google.dev/gemini-api/docs/url-context
- Structured Output: https://ai.google.dev/gemini-api/docs/structured-output
- Safety Settings: https://ai.google.dev/gemini-api/docs/safety-setting
