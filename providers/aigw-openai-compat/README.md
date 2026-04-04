# aigw-openai-compat

## 目标

`aigw-openai-compat` 用来承接“兼容 OpenAI API 形状，但并非 OpenAI 官方”的 provider。

典型对象：

- vLLM
- Together AI
- Groq
- Fireworks
- Perplexity
- LM Studio

这些 provider 大多复用了 OpenAI 风格的路径、JSON 结构和 SSE 格式，但通常存在差异：

- `base_url` 不同
- 支持的 endpoint 子集不同
- `Responses API` 往往不完整甚至没有
- `tools`、`tool_choice`、`vision`、`parallel_tool_calls` 支持度不同
- 某些字段虽然接受，但会静默忽略

所以这里不应该复用 `aigw-openai` 作为“第三方兼容总线”，而应单独做一个可配置 compat 层。

## 职责边界

- `aigw-openai`
  - 只负责 OpenAI 官方标准
  - 目标是最大限度协议忠实
- `aigw-openai-compat`
  - 负责 OpenAI-compatible 生态
  - 允许通过 `quirks` 描述第三方差异
  - 默认只承诺“OpenAI-shaped”，不承诺“OpenAI-identical”

## 当前骨架

目前先落最小配置模型：

- `OpenAICompatProvider`
- `OpenAICompatConfig`
- `Quirks`
- `OpenAICompatConfigError`

后续实现建议：

1. 抽一个共享 HTTP/SSE transport 层给 `aigw-openai` 和 `aigw-openai-compat` 共用。
2. `compat` 层优先支持：
   - `chat/completions`
   - `embeddings`
   - 常见流式 chat SSE
3. `Responses API` 支持放到能力探测或 provider preset 里，不要默认假设存在。
4. 所有 provider 特性差异都经由 `quirks` 或 capability matrix 表达，不要把 Groq/Together/vLLM 的分支硬编码进官方 OpenAI crate。
