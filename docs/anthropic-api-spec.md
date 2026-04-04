# Anthropic Messages API Specification

Reference for implementing the Anthropic provider. Based on the official API docs at `platform.claude.com/docs/en/api/messages`.

## Endpoint

```
POST https://api.anthropic.com/v1/messages
```

### Required Headers

```
Content-Type: application/json
x-api-key: $ANTHROPIC_API_KEY
anthropic-version: 2023-06-01
```

## Request Schema

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `model` | `string` | Model identifier, e.g. `"claude-opus-4-6"`, `"claude-sonnet-4-6"` |
| `max_tokens` | `integer` | Maximum output tokens (**required**, unlike OpenAI) |
| `messages` | `MessageParam[]` | Conversation messages |

### Optional Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `system` | `string \| TextBlockParam[]` | -- | System prompt (separate from messages) |
| `temperature` | `number` | `1.0` | 0.0-1.0 |
| `top_p` | `number` | -- | Nucleus sampling |
| `top_k` | `number` | -- | Top-K sampling |
| `stop_sequences` | `string[]` | -- | Custom stop sequences |
| `stream` | `boolean` | `false` | Enable SSE streaming |
| `tools` | `ToolUnion[]` | -- | Available tools |
| `tool_choice` | `ToolChoice` | -- | Tool selection strategy |
| `metadata` | `{ user_id?: string }` | -- | Request metadata |
| `thinking` | `ThinkingConfigParam` | -- | Extended thinking config |

### MessageParam

```typescript
{
  role: "user" | "assistant",   // No "system" role — system is a top-level field
  content: string | ContentBlockParam[]
}
```

Messages must strictly alternate between `user` and `assistant`.

### Content Block Types (Request)

#### TextBlockParam
```json
{ "type": "text", "text": "string" }
```

#### ImageBlockParam
```json
{
  "type": "image",
  "source": {
    "type": "base64",
    "media_type": "image/jpeg | image/png | image/gif | image/webp",
    "data": "base64-encoded-string"
  }
}
```

Or URL source:
```json
{
  "type": "image",
  "source": { "type": "url", "url": "https://..." }
}
```

#### ToolUseBlockParam (echoing back assistant's tool call)
```json
{
  "type": "tool_use",
  "id": "toolu_01T1x...",
  "name": "get_weather",
  "input": { "location": "SF" }
}
```

#### ToolResultBlockParam (user returning tool result)
```json
{
  "type": "tool_result",
  "tool_use_id": "toolu_01T1x...",
  "content": "72F sunny",
  "is_error": false
}
```

#### ThinkingBlockParam (echoing back thinking)
```json
{
  "type": "thinking",
  "thinking": "...",
  "signature": "EqQBCgIYAh..."
}
```

### Tool Definition

```json
{
  "name": "get_weather",
  "description": "Get current weather",
  "input_schema": {
    "type": "object",
    "properties": {
      "location": { "type": "string" }
    },
    "required": ["location"]
  }
}
```

### ToolChoice

```json
{ "type": "auto" }
{ "type": "any" }
{ "type": "tool", "name": "get_weather" }
{ "type": "none" }
```

All except `"none"` support `disable_parallel_tool_use?: boolean`.

### ThinkingConfigParam

```json
{ "type": "enabled", "budget_tokens": 10000 }
{ "type": "disabled" }
{ "type": "adaptive" }
```

## Response Schema

```json
{
  "id": "msg_1234567890",
  "type": "message",
  "role": "assistant",
  "content": [
    { "type": "text", "text": "Hello!" }
  ],
  "model": "claude-opus-4-6",
  "stop_reason": "end_turn",
  "stop_sequence": null,
  "usage": {
    "input_tokens": 25,
    "output_tokens": 15,
    "cache_creation_input_tokens": 0,
    "cache_read_input_tokens": 0
  }
}
```

### Response Content Block Types

**TextBlock**: `{ "type": "text", "text": "..." }`

**ToolUseBlock**: `{ "type": "tool_use", "id": "toolu_xxx", "name": "get_weather", "input": { "location": "SF" } }`

**ThinkingBlock**: `{ "type": "thinking", "thinking": "...", "signature": "..." }`

### Stop Reasons

| Value | Meaning |
|-------|---------|
| `end_turn` | Model finished naturally |
| `max_tokens` | Hit max_tokens limit |
| `stop_sequence` | Hit custom stop sequence |
| `tool_use` | Model initiated tool call |

## Streaming Events

With `"stream": true`, response is SSE. Event order:

```
message_start
  content_block_start  (index 0)
    content_block_delta (text_delta / input_json_delta / thinking_delta)
    ...
  content_block_stop   (index 0)
  content_block_start  (index 1)
    ...
  content_block_stop   (index 1)
message_delta          (stop_reason, usage)
message_stop

ping can appear anywhere
```

### message_start

```
event: message_start
data: {"type":"message_start","message":{"id":"msg_xxx","type":"message","role":"assistant","content":[],"model":"claude-opus-4-6","stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":25,"output_tokens":1}}}
```

### content_block_start

Text:
```
event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}
```

Tool use:
```
event: content_block_start
data: {"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"toolu_xxx","name":"get_weather","input":{}}}
```

### content_block_delta

Text delta:
```
event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}
```

Tool input JSON delta:
```
event: content_block_delta
data: {"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"location\": \"San Fra"}}
```

Thinking delta:
```
event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"I need to find..."}}
```

### content_block_stop

```
event: content_block_stop
data: {"type":"content_block_stop","index":0}
```

### message_delta

```
event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":15}}
```

Note: `usage.output_tokens` here is **cumulative**, not incremental.

### message_stop

```
event: message_stop
data: {"type":"message_stop"}
```

### error (in-stream)

```
event: error
data: {"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}
```
