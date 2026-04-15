# Provider Translation Reference

Field-level mapping between the unified OpenAI-compatible format and each provider's native format.

## OpenAI & OpenAI-Compatible Providers

### `aigw-openai` — Official OpenAI

Near-passthrough. The unified format **is** the OpenAI format, so no translation needed. The crate handles:
- Building HTTP requests with OpenAI-specific headers (`Authorization: Bearer`, `OpenAI-Organization`, `OpenAI-Project`)
- SSE stream parsing (`data: {...}` lines, `data: [DONE]` terminator)
- Response deserialization into unified types

### `aigw-openai` — Responses API Translation

Translates canonical Chat Completions format ↔ OpenAI Responses API (`/v1/responses`). Used when the gateway receives Chat Completions requests but needs to forward them via the Responses API (e.g. for Codex CLI compatibility, or when Responses API features like `previous_response_id` are needed).

#### Request Translation (Chat Completions → Responses API)

| Chat Completions | Responses API | Notes |
|---|---|---|
| `model` | `model` | Direct |
| `messages` (role: system/developer) | `instructions` **or** `input[]` role: developer | `SystemHandling::ExtractToInstructions` (default) vs `MapToDeveloper` |
| `messages` (role: user) | `input[]: { type: "message", role: "user", content: [...] }` | Content parts: `text` → `input_text`, `image_url` → `input_image` |
| `messages` (role: assistant) | `input[]: { type: "message", role: "assistant", content: [...] }` | Content parts: `text` → `output_text` |
| `messages` (role: assistant + tool_calls) | `input[]: { type: "function_call", call_id, name, arguments }` | Each tool_call becomes a separate input item |
| `messages` (role: tool) | `input[]: { type: "function_call_output", call_id, output }` | |
| `max_tokens` | `max_output_tokens` | Dropped when `drop_max_tokens = true` (Codex backend) |
| `temperature` | `temperature` | Dropped when `drop_temperature = true` |
| `top_p` | `top_p` | Dropped when `drop_top_p = true` |
| `tools[].function` | `tools[]: { type: "function", name, description, parameters, strict }` | Flattened (no nested `function` wrapper) |
| `tool_choice` | `tool_choice` | Named: `{ type: "function", function: { name } }` → `{ type: "function", name }` |
| `reasoning_effort` (extra) | `reasoning.effort` | Chat Completions shorthand |
| `reasoning` (extra, object) | `reasoning` | Pass-through, merged with `reasoning_effort` |
| `response_format: { type: "json_schema", json_schema: {...} }` | `text.format: { type: "json_schema", name, schema, strict }` | Fields flattened to top level |
| `store` (extra) | `store` | Falls back to `default_store` config |
| `include` (extra) | `include` | Falls back to `default_include` config |
| `parallel_tool_calls` (extra) | `parallel_tool_calls` | Falls back to `default_parallel_tool_calls` config |

#### `ResponsesRequestConfig` Presets

| Config | Public API (`Default`) | Codex backend (`::codex()`) |
|---|---|---|
| `system_handling` | `ExtractToInstructions` | `MapToDeveloper` |
| `drop_max_tokens` | `false` | `true` |
| `drop_temperature` | `false` | `true` |
| `drop_top_p` | `false` | `true` |
| `default_store` | `None` | `Some(false)` |
| `default_include` | `None` | `Some(["reasoning.encrypted_content"])` |
| `default_parallel_tool_calls` | `None` | `Some(true)` |
| `default_reasoning_summary` | `None` | `Some("auto")` |
| `default_reasoning_effort` | `None` | `Some("medium")` |
| `force_instructions` | `false` | `true` |
| `max_tool_name_len` | `None` | `Some(64)` |

#### Response Translation (Responses API → Chat Completions)

| Responses API | Chat Completions | Notes |
|---|---|---|
| `id` | `id` | Direct |
| `model` | `model` | Direct |
| `created_at` | `created` | Direct |
| `output[type=message].content[type=output_text]` | `choices[0].message.content` | Concatenated |
| `output[type=function_call]` | `choices[0].message.tool_calls[]` | `call_id` → `id`, flattened to `{ id, type: "function", function: { name, arguments } }` |
| `output[type=reasoning].summary[type=summary_text]` | `choices[0].message.extra.reasoning_content` | Via Message.extra flatten |
| `status: "completed"` | `finish_reason: "stop"` / `"tool_calls"` | `tool_calls` if any function_call items |
| `status: "incomplete"` | `finish_reason: "length"` | |
| `usage.input_tokens` | `usage.prompt_tokens` | |
| `usage.output_tokens` | `usage.completion_tokens` | |
| `usage.input_tokens_details` | `usage.extra.prompt_tokens_details` | |
| `usage.output_tokens_details` | `usage.extra.completion_tokens_details` | |

#### Streaming Translation (Responses API SSE → Canonical StreamEvent)

| Responses API SSE event | StreamEvent |
|---|---|
| `response.created` | `ResponseMeta { id, model }` |
| `response.output_text.delta` | `ContentDelta(text)` |
| `response.reasoning_summary_text.delta` | `ReasoningDelta(text)` |
| `response.reasoning_summary_text.done` | `ReasoningDelta("\n\n")` (segment separator) |
| `response.output_item.added` (type: reasoning) | Buffer `encrypted_content` |
| `response.output_item.done` (type: reasoning) | `ReasoningSignature(encrypted_content)` |
| `response.output_item.added` (type: function_call) | `ToolCallStart { index, id, name }` (flushes buffered signature first) |
| `response.function_call_arguments.delta` | `ToolCallDelta { index, arguments }` |
| `response.completed` | `Finish(reason)` + `Usage(...)` + `Done` |
| `[DONE]` | `Done` |

### `aigw-openai-compat` — OpenAI-Compatible Third Parties

Same wire format as OpenAI, but configurable:
- `base_url` substitution (the only hard requirement)
- `Quirks` flags to declare which features the provider actually supports
- `default_headers` for provider-specific auth patterns
- Request sanitization: strip unsupported fields based on `Quirks` before sending

**Pre-send sanitization based on Quirks:**

| Quirk | When `false` |
|-------|-------------|
| `supports_tool_choice` | Strip `tool_choice` field from request |
| `supports_parallel_tool_calls` | Strip `parallel_tool_calls` field |
| `supports_vision` | Reject requests with image content parts (return error) |
| `supports_streaming` | Reject `stream: true` requests |
| `supports_responses_api` | Only route to `chat/completions`, never `responses` |

**Covered providers (non-exhaustive):** vLLM, Together AI, Groq, Fireworks, Perplexity, LM Studio, Ollama, DeepSeek, Mistral (La Plateforme), xAI (Grok)

## Anthropic Translation

### Request Translation (OpenAI → Anthropic)

#### Top-Level Fields

| OpenAI (unified) | Anthropic | Notes |
|-------------------|-----------|-------|
| `model` | `model` | Direct passthrough |
| `messages` | `messages` + `system` | Extract `role: "system"` messages into top-level `system` field |
| `temperature` | `temperature` | Direct |
| `top_p` | `top_p` | Direct |
| `max_tokens` | `max_tokens` | **Required** in Anthropic. Provide default (e.g. 4096) if absent |
| `stream` | `stream` | Direct |
| `stop` | `stop_sequences` | Rename; normalize string to `string[]` |
| `n` | -- | Not supported. Reject or ignore if `n > 1` |
| `tools` | `tools` | Translate tool format (see below) |
| `tool_choice` | `tool_choice` | Translate format (see below) |
| `response_format` | -- | `{ type: "json_object" }` has no direct equivalent |

#### Message Translation

```
OpenAI                              → Anthropic
─────────────────────────────────────────────────
role: "system"                      → Extract to top-level `system` field
role: "user"                        → role: "user"
role: "assistant"                   → role: "assistant"
role: "tool"                        → role: "user", content: [{ type: "tool_result", ... }]
```

#### Content Translation

```
OpenAI                              → Anthropic
─────────────────────────────────────────────────
content: "string"                   → content: "string"  (direct)
content: [{ type: "text", ... }]    → content: [{ type: "text", ... }]  (direct)
content: [{ type: "image_url",      → content: [{ type: "image",
  image_url: { url } }]                source: { type: "url", url } }]
                                      or base64 decode if data: URI
```

#### Tool Definition Translation

```
OpenAI                              → Anthropic
─────────────────────────────────────────────────
{                                   → {
  type: "function",                 →   // no type field
  function: {                       →
    name: "get_weather",            →   name: "get_weather",
    description: "...",             →   description: "...",
    parameters: { ... }             →   input_schema: { ... }
  }                                 →
}                                   → }
```

#### Tool Choice Translation

```
OpenAI                              → Anthropic
─────────────────────────────────────────────────
"auto"                              → { type: "auto" }
"none"                              → { type: "none" }
"required"                          → { type: "any" }
{ type: "function",                 → { type: "tool",
  function: { name: "X" } }        →   name: "X" }
```

#### Tool Call in Assistant Message

```
OpenAI (assistant message)          → Anthropic (assistant message)
─────────────────────────────────────────────────
message.content = "text"            → content: [{ type: "text", text: "text" }]
message.tool_calls = [{             →            + [{ type: "tool_use",
  id: "call_xxx",                   →              id: "call_xxx",
  type: "function",                 →              name: "get_weather",
  function: {                       →              input: { "location": "SF" }
    name: "get_weather",            →            }]
    arguments: '{"location":"SF"}'  →   // arguments: parsed from JSON string to object
  }                                 →
}]                                  →
```

#### Tool Result Translation

```
OpenAI (tool message)               → Anthropic (user message with tool_result)
─────────────────────────────────────────────────
{                                   → {
  role: "tool",                     →   role: "user",
  tool_call_id: "call_xxx",        →   content: [{
  content: "72F sunny"             →     type: "tool_result",
}                                   →     tool_use_id: "call_xxx",
                                    →     content: "72F sunny"
                                    →   }]
                                    → }
```

Note: Multiple consecutive `role: "tool"` messages should be merged into a single `role: "user"` message with multiple `tool_result` blocks.

### Response Translation (Anthropic → OpenAI)

#### Top-Level

```
Anthropic                           → OpenAI
─────────────────────────────────────────────────
{                                   → {
  id: "msg_xxx",                    →   id: "msg_xxx",
  type: "message",                  →   object: "chat.completion",
  role: "assistant",                →   created: <unix timestamp>,
  content: [...],                   →   model: "...",
  model: "...",                     →   choices: [{
  stop_reason: "...",               →     index: 0,
  usage: { ... }                    →     message: { role: "assistant", ... },
}                                   →     finish_reason: "..."
                                    →   }],
                                    →   usage: { ... }
                                    → }
```

#### Content Block → Message

Anthropic response `content` is always an array of blocks. Translation:

1. Collect all `{ type: "text" }` blocks → concatenate into `message.content` string
2. Collect all `{ type: "tool_use" }` blocks → convert to `message.tool_calls` array
3. Ignore `thinking` / `redacted_thinking` blocks (or map to provider-specific extension)

```
Anthropic content blocks            → OpenAI message
─────────────────────────────────────────────────
[                                   → {
  { type: "text", text: "Let me" }, →   role: "assistant",
  { type: "text", text: " check" },→   content: "Let me check",
  { type: "tool_use",              →   tool_calls: [{
    id: "toolu_xxx",               →     id: "toolu_xxx",
    name: "get_weather",           →     type: "function",
    input: { location: "SF" }      →     function: {
  }                                →       name: "get_weather",
]                                   →       arguments: "{\"location\":\"SF\"}"
                                    →     }
                                    →   }]
                                    → }
```

If content blocks are only text: `tool_calls` is omitted.
If content blocks are only tool_use: `content` is `null`.

#### Stop Reason Mapping

| Anthropic `stop_reason` | OpenAI `finish_reason` |
|--------------------------|------------------------|
| `end_turn` | `stop` |
| `max_tokens` | `length` |
| `stop_sequence` | `stop` |
| `tool_use` | `tool_calls` |

#### Usage Mapping

```
Anthropic                           → OpenAI
─────────────────────────────────────────────────
usage.input_tokens                  → usage.prompt_tokens
usage.output_tokens                 → usage.completion_tokens
                                    → usage.total_tokens = prompt + completion
usage.cache_creation_input_tokens   → (drop or pass in extra)
usage.cache_read_input_tokens       → (drop or pass in extra)
```

### Streaming Translation (Anthropic → OpenAI)

Anthropic uses named SSE events with block-level granularity. OpenAI uses unnamed `data:` lines with choice-level deltas.

#### Event Mapping

```
Anthropic Event                     → OpenAI Chunk
─────────────────────────────────────────────────────────────────

message_start                       → chunk { choices: [{ delta: { role: "assistant" }, index: 0 }] }
                                      (first chunk establishes role)

content_block_start (text)          → (no output — wait for deltas)

content_block_delta (text_delta)    → chunk { choices: [{ delta: { content: "text" }, index: 0 }] }

content_block_stop (text)           → (no output)

content_block_start (tool_use)      → chunk { choices: [{ delta: {
                                        tool_calls: [{ index: N, id: "toolu_xxx",
                                        type: "function", function: { name: "get_weather", arguments: "" } }]
                                      }, index: 0 }] }

content_block_delta (input_json)    → chunk { choices: [{ delta: {
                                        tool_calls: [{ index: N,
                                        function: { arguments: "partial_json" } }]
                                      }, index: 0 }] }

content_block_stop (tool_use)       → (no output)

message_delta                       → chunk { choices: [{ delta: {},
                                        finish_reason: "stop|tool_calls|length", index: 0 }],
                                        usage: { ... } }

message_stop                        → data: [DONE]

ping                                → (ignore)
```

#### Streaming State Machine

The translator must maintain state to track:

1. **Current tool call index** — incremented on each `content_block_start` with `type: "tool_use"`
2. **Message ID and model** — captured from `message_start`, reused in all chunks
3. **Created timestamp** — generated once at `message_start`

## Google Gemini Translation

### Request Translation (OpenAI → Gemini)

#### Endpoint

```
POST https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={API_KEY}
```

For streaming:
```
POST https://generativelanguage.googleapis.com/v1beta/models/{model}:streamGenerateContent?alt=sse&key={API_KEY}
```

#### Top-Level Structure

```
OpenAI                              → Gemini
─────────────────────────────────────────────────
{                                   → {
  model: "gemini-2.5-pro",         →   // model is in the URL path
  messages: [...],                  →   contents: [...],
  temperature: 0.7,                →   generationConfig: {
  max_tokens: 1000,                →     temperature: 0.7,
  top_p: 0.9,                     →     maxOutputTokens: 1000,
  stop: ["END"],                   →     topP: 0.9,
                                    →     stopSequences: ["END"],
                                    →   },
  tools: [...],                    →   tools: [{ functionDeclarations: [...] }],
}                                   →   systemInstruction: { ... }
                                    → }
```

#### Message/Content Translation

```
OpenAI                              → Gemini
─────────────────────────────────────────────────
role: "system"                      → systemInstruction: { parts: [{ text: "..." }] }
role: "user"                        → role: "user"
role: "assistant"                   → role: "model"
role: "tool"                        → role: "function"
```

Content structure:
```
OpenAI message                      → Gemini content
─────────────────────────────────────────────────
{ role, content: "text" }           → { role, parts: [{ text: "text" }] }
{ role, content: [                  → { role, parts: [
  { type: "text", text },          →   { text },
  { type: "image_url",             →   { inlineData: {
    image_url: { url } }           →       mimeType: "...", data: "base64" } }
]}                                  → ]}
```

#### Tool Definition Translation

```
OpenAI                              → Gemini
─────────────────────────────────────────────────
tools: [{                           → tools: [{ functionDeclarations: [{
  type: "function",                 →   name: "get_weather",
  function: {                       →   description: "...",
    name: "get_weather",            →   parameters: { ... }
    description: "...",             → }]}]
    parameters: { ... }             →
  }                                 →
}]                                  →
```

#### Tool Call in Response

```
Gemini                              → OpenAI
─────────────────────────────────────────────────
parts: [{                           → tool_calls: [{
  functionCall: {                   →   id: "<generate-uuid>",
    name: "get_weather",            →   type: "function",
    args: { location: "SF" }        →   function: {
  }                                 →     name: "get_weather",
}]                                  →     arguments: "{\"location\":\"SF\"}"
                                    →   }
                                    → }]
```

Note: Gemini does not provide tool call IDs. Generate a UUID.

#### Tool Result

```
OpenAI                              → Gemini
─────────────────────────────────────────────────
{                                   → {
  role: "tool",                     →   role: "function",
  tool_call_id: "xxx",             →   parts: [{
  content: "72F sunny"             →     functionResponse: {
}                                   →       name: "get_weather",
                                    →       response: { result: "72F sunny" }
                                    →     }
                                    →   }]
                                    → }
```

### Response Translation (Gemini → OpenAI)

```
Gemini                              → OpenAI
─────────────────────────────────────────────────
{                                   → {
  candidates: [{                    →   id: "<generate>",
    content: {                      →   object: "chat.completion",
      role: "model",               →   created: <timestamp>,
      parts: [                      →   model: "gemini-2.5-pro",
        { text: "Hello" }          →   choices: [{
      ]                             →     index: 0,
    },                              →     message: {
    finishReason: "STOP",           →       role: "assistant",
  }],                               →       content: "Hello"
  usageMetadata: {                  →     },
    promptTokenCount: 10,           →     finish_reason: "stop"
    candidatesTokenCount: 5,        →   }],
    totalTokenCount: 15             →   usage: {
  }                                 →     prompt_tokens: 10,
}                                   →     completion_tokens: 5,
                                    →     total_tokens: 15
                                    →   }
                                    → }
```

#### Finish Reason Mapping

| Gemini `finishReason` | OpenAI `finish_reason` |
|-----------------------|------------------------|
| `STOP` | `stop` |
| `MAX_TOKENS` | `length` |
| `SAFETY` | `content_filter` |
| `RECITATION` | `content_filter` |
| `OTHER` | `stop` |

### Streaming Translation (Gemini → OpenAI)

Gemini streaming returns complete `GenerateContentResponse` objects per SSE event (each containing `candidates` with accumulated or partial `parts`). Each event is a snapshot.

```
Gemini SSE                          → OpenAI Chunk
─────────────────────────────────────────────────
data: { candidates: [{              → data: { id, object: "chat.completion.chunk",
  content: { parts: [{ text }] },  →   choices: [{ index: 0,
  finishReason: null               →     delta: { content: "text" },
}], usageMetadata: ... }           →     finish_reason: null }] }

(last event with finishReason)      → data: { ..., choices: [{ ...,
                                    →     finish_reason: "stop" }],
                                    →     usage: { ... } }

(end of stream)                     → data: [DONE]
```

For text: diff consecutive `parts[].text` to produce incremental deltas.
For function calls: emit tool_calls delta on first appearance.
