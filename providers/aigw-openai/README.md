# aigw-openai

## 目标

`aigw-openai` 应该实现成一个“协议忠实”的 OpenAI Provider：

- 对外优先保持 OpenAI 官方请求/响应形状，而不是先做跨厂商抽象。
- 以最新官方标准为准，不以过时的第三方实现为准。
- 先把 wire format 做准，再在更高层做统一能力封装。

## 核心结论

- 新实现应以 `Responses API` 为一等公民，`Chat Completions API` 为兼容层。OpenAI 官方文档和官方 SDK 都明确把 `Responses API` 作为新项目首选。
- 规范源不能只看 `openai-openapi` 的 `manual_spec` 分支。它会滞后于当前文档和官方 SDK。更可靠的来源是：
  - 官方 API 文档
  - 官方 JavaScript SDK
  - 官方 Python SDK
  - `openai-openapi` README 指向的 documented OpenAPI spec
- Rust 实现不建议“全自动生成到底”。OpenAI spec 是 OpenAPI 3.1，且 `oneOf`/`anyOf`/流式事件联合非常多，直接代码生成通常会得到很差的 Rust 类型。更稳的方案是：
  - 用 spec 驱动接口面和字段校验
  - 手写核心 wire types
  - 少量 convenience helper 单独放在解析层

## 官方依据

- API Reference: https://platform.openai.com/docs/api-reference
- Responses API: https://platform.openai.com/docs/api-reference/responses
- Chat Completions API: https://platform.openai.com/docs/api-reference/chat
- Structured Outputs: https://platform.openai.com/docs/guides/structured-outputs
- Function Calling: https://platform.openai.com/docs/guides/function-calling
- Conversation State: https://platform.openai.com/docs/guides/conversation-state
- Prompt Caching: https://platform.openai.com/docs/guides/prompt-caching
- openai-node: https://github.com/openai/openai-node
- openai-python: https://github.com/openai/openai-python
- openai-openapi README: https://github.com/openai/openai-openapi
- documented spec: https://app.stainless.com/api/spec/documented/openai/openapi.documented.yml

## 推荐支持面

如果目标是“完全符合 OpenAI 标准”，建议按下面顺序推进，而不是只做 `/chat/completions`。

### P0

- `POST /v1/responses`
- `GET /v1/responses/{response_id}`
- `POST /v1/responses/{response_id}/cancel`
- `GET /v1/responses/{response_id}/input_items`
- `POST /v1/responses/input_tokens`
- `POST /v1/responses/compact`
- `POST /v1/chat/completions`
- `POST /v1/embeddings`
- `GET /v1/models`
- `GET /v1/models/{model}`

### P1

- `GET /v1/chat/completions/{completion_id}`
- `POST /v1/chat/completions/{completion_id}`
- `GET /v1/chat/completions`
- `DELETE /v1/chat/completions/{completion_id}`
- `GET /v1/chat/completions/{completion_id}/messages`
- `POST /v1/images/generations`
- `POST /v1/audio/transcriptions`
- `POST /v1/audio/speech`
- `POST /v1/moderations`

### P2

- `POST /v1/files`
- `GET /v1/files`
- `GET /v1/files/{file_id}`
- `GET /v1/files/{file_id}/content`
- `POST /v1/vector_stores`
- `GET /v1/vector_stores/...`
- 其他你网关确实要暴露的 OpenAI 资源

## 不建议的方向

- 不要把 `Chat Completions` 当成内部主协议，再“翻译”出 `Responses`。
- 不要试图先定义一个统一的 `Message`/`ToolCall` 跨厂商模型，然后再映射 OpenAI。OpenAI 的 input/output/tool union 已经足够复杂，先归一化只会丢语义。
- 不要只参考 `manual_spec` 分支。
- 不要丢弃未知字段。OpenAI 的文档、SDK、规范并不总是完全同步。
- 不要把第三方 OpenAI-compatible provider 的差异分支直接写进 `aigw-openai`。这类能力应该下沉到独立的 `aigw-openai-compat` crate。

## 与 openai-compat 的关系

这两个 crate 的职责必须分开：

- `aigw-openai`
  - 面向 OpenAI 官方 API
  - 追求“协议忠实”
  - 默认不接受第三方 provider 的非标准行为污染
- `aigw-openai-compat`
  - 面向 Groq、Together、vLLM、Fireworks、Perplexity、LM Studio 一类“OpenAI-compatible” provider
  - 通过 `base_url + quirks` 建模差异
  - 默认只保证“OpenAI-shaped”，不保证“OpenAI-identical”

设计原则：

- 能共享 transport，就共享 transport。
- 能共享 wire type，就共享 wire type。
- 但 provider capability、quirks、fallback 行为不要反向侵入官方 OpenAI crate。

## 推荐 crate 结构

```text
providers/aigw-openai/
├── src/
│   ├── lib.rs
│   ├── client.rs
│   ├── error.rs
│   ├── sse.rs
│   ├── resources/
│   │   ├── responses.rs
│   │   ├── chat_completions.rs
│   │   ├── embeddings.rs
│   │   ├── models.rs
│   │   ├── images.rs
│   │   ├── audio.rs
│   │   ├── moderations.rs
│   │   ├── files.rs
│   │   └── vector_stores.rs
│   └── types/
│       ├── shared.rs
│       ├── responses.rs
│       ├── chat.rs
│       ├── embeddings.rs
│       ├── models.rs
│       ├── images.rs
│       ├── audio.rs
│       ├── moderations.rs
│       ├── files.rs
│       └── vector_stores.rs
```

这里最重要的原则是：

- `resources/*` 负责 endpoint 行为。
- `types/*` 负责 OpenAI 原始协议对象。
- `sse.rs` 只负责 SSE 解析，不夹带业务逻辑。
- “自动解析 JSON schema 输出”或“自动解析 function arguments”这种 helper，单独放在 convenience 层，不要污染原始类型。

## 协议实现最佳实践

### 1. Headers 和客户端选项

- 默认 base URL: `https://api.openai.com/v1`
- 认证头：`Authorization: Bearer <api_key>`
- 额外透传：
  - `OpenAI-Organization`
  - `OpenAI-Project`
- 客户端应支持：
  - 全局 timeout
  - 单请求 timeout
  - 全局 retry
  - 单请求 retry
  - 自定义 headers/query/body 透传

### 2. 错误模型

官方 SDK 的错误分类很稳定，Rust 里建议保持同样语义：

- `400 -> BadRequest`
- `401 -> Authentication`
- `403 -> PermissionDenied`
- `404 -> NotFound`
- `409 -> Conflict`
- `422 -> UnprocessableEntity`
- `429 -> RateLimit`
- `>=500 -> InternalServerError`
- 无响应/网络错误 -> ConnectionError
- 超时 -> TimeoutError

同时保留这些字段：

- `status`
- `message`
- `type`
- `param`
- `code`
- `x-request-id`
- 原始响应 body

### 3. Retry 策略

官方 JS/Python SDK 的默认行为基本一致：

- 默认重试次数：`2`
- 默认重试场景：
  - 网络错误
  - `408`
  - `409`
  - `429`
  - `>=500`
- 优先尊重：
  - `x-should-retry`
  - `retry-after-ms`
  - `retry-after`
- 默认退避：
  - 初始 `0.5s`
  - 最大 `8s`
  - 带 `25%` jitter

### 4. Responses 是主轴

`Responses` 不是 “另一个聊天接口”，而是当前 OpenAI 的主协议面。实现时至少要完整覆盖这些语义：

- `input`
- `instructions`
- `previous_response_id`
- `conversation`
- `context_management`
- `text.format`
- `tools`
- `tool_choice`
- `parallel_tool_calls`
- `reasoning`
- `max_output_tokens`
- `truncation`
- `include`
- `background`
- `stream`
- `stream_options`
- `store`
- `metadata`
- `safety_identifier`
- `prompt_cache_key`
- `prompt_cache_retention`

关键语义：

- `previous_response_id` 用于多轮上下文。
- `previous_response_id` 和 `conversation` 不能混用。
- 当同时使用 `previous_response_id` 和 `instructions` 时，上一轮的 instructions 不会自动继承。
- `user` 已经是被替换路径，新的实现应优先支持：
  - `safety_identifier`
  - `prompt_cache_key`
  - `prompt_cache_retention`

### 5. Chat Completions 作为兼容层

Chat Completions 仍然要做准，但定位应该很明确：

- 它是兼容接口，不是内部中心模型。
- 必须支持 `developer` role，不要只支持 `system/user/assistant/tool`。
- 不要为了复用代码，把 Chat 请求强行翻成 Responses 再发出去。两者虽然语义接近，但对象形状、工具、stream chunk、finish reason 和存储能力并不完全一致。

### 6. Structured Outputs 和 Function Calling

- 对文本结构化输出，优先支持 `text.format = { type: "json_schema", ... }`
- `json_object` 只作为旧模式兼容
- Function tool 的 `parameters` 必须保留为 JSON Schema
- `strict` 必须原样支持

这里有一个很重要的现实细节：

- 当前 API reference 页面和 documented spec 对 `strict` 的默认值存在不一致迹象
- 所以实现里不要推断默认值
- 最稳妥的策略是：
  - 序列化时按调用方显式值输出
  - 反序列化时原样接收
  - 在上层 helper 中建议调用方显式传 `strict: true`

### 7. Tool 联合类型必须做全

如果目标真的是“完全符合 OpenAI 标准”，`tools` 和 `tool_choice` 不能只做 function + web search 两种。

至少要按官方类型联合建模，哪怕某些资源后续才真正联通：

- function
- file_search
- web_search_preview
- image_generation
- code_interpreter
- computer_use
- mcp
- custom
- apply_patch
- shell

实现建议：

- 原始协议层把 union 全建出来
- 尚未真正支持的工具类型，可以在发送前返回明确的 “not implemented”
- 但不要在类型层直接删掉这些变体

### 8. 流式响应要“逐事件忠实”

`Responses` 的流式不是简单的纯文本 delta，而是一组事件：

- `response.created`
- `response.in_progress`
- `response.output_item.added`
- `response.content_part.added`
- `response.output_text.delta`
- `response.output_text.done`
- `response.function_call_arguments.delta`
- `response.completed`
- 以及其他工具相关事件

实现要求：

- 暴露原始 SSE event
- 保持事件顺序
- 不要只输出聚合文本
- 可以额外提供 snapshot helper，但不能替代原始事件流
- `GET /responses/{response_id}?stream=true&starting_after=...` 要支持断点续流

还要注意一个当前规范里的细节：

- `stream_options.include_obfuscation` 存在
- delta 事件里可能出现额外 `obfuscation` 字段
- 所以流式事件对象要允许 forward-compatible extra fields

### 9. 原始类型要保留未知字段

这是 Rust 里最容易被忽略、但对 OpenAI 兼容性最重要的一点。

建议所有核心 request/response/event object 都预留：

```rust
#[serde(flatten)]
pub extra: std::collections::BTreeMap<String, serde_json::Value>;
```

尤其是这些对象：

- `Response`
- `ResponseStreamEvent`
- `ChatCompletion`
- `ChatCompletionChunk`
- 各类 `Tool` 和 `ToolChoice`

原因很简单：

- 官方 SDK 明确允许 undocumented params/fields 透传
- 官方文档、spec、SDK 之间存在时差
- 如果你把 schema 写死，Provider 很容易在一次 OpenAI 小更新后就不兼容

### 10. Convenience helper 只能加，不要改协议

可以加这些 helper：

- `output_text()` 聚合所有 `output_text`
- `parsed_output<T>()` 解析 `json_schema`
- `parsed_function_arguments()` 解析 strict function arguments
- `request_id()` 读取 `x-request-id`

但不要做这些事情：

- 删除原始 `output`
- 把多个 output item 合成一个自定义 message
- 把原始 stream event 变成只剩 token 文本

## Rust 落地建议

- HTTP 客户端：`reqwest`
- JSON：`serde` + `serde_json`
- SSE：自己基于字节流解析，或者在确保不丢事件字段的前提下使用成熟 SSE parser
- 错误：自定义 error enum，但保留 OpenAI 原始 error body
- 分页：按 endpoint 单独实现，不要做过度抽象
- 测试：优先做 snapshot test 和 wire-format test

## 测试建议

- 用官方文档示例做 JSON round-trip 测试
- 用官方 spec 中的 Responses streaming 示例做 SSE 回放测试
- 覆盖：
  - 非流式 `responses.create`
  - 流式 `responses.create`
  - `responses.retrieve(..., stream=true, starting_after=...)`
  - `chat.completions.create`
  - `embeddings.create`
  - 429/500/retry-after 退避
  - `x-request-id` 提取
  - `developer` role
  - `json_schema` 结构化输出
  - strict function calling

## 最终建议

对 `aigw-openai`，最稳的路线不是“先做一个大而统一的网关抽象”，而是：

1. 先把 OpenAI 原始协议完整实现。
2. 先做 `Responses`，再做 `Chat Completions` 兼容。
3. 让类型层对新字段天然宽容。
4. 用官方 documented spec 做变更检测。
5. convenience helper 和协议层彻底分离。

这样后续即使再加 Anthropic/Gemini 的统一抽象，也不会反过来污染 OpenAI Provider 的准确性。
