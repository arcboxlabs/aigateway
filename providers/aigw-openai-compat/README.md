# aigw-openai-compat

## Goal

`aigw-openai-compat` handles providers that are "OpenAI API-shaped but not officially OpenAI."

Typical targets:

- vLLM
- Together AI
- Groq
- Fireworks
- Perplexity
- LM Studio

These providers largely reuse OpenAI-style paths, JSON structures, and SSE formats, but usually diverge in some ways:

- Different `base_url`
- Different supported endpoint subsets
- `Responses API` is often incomplete or absent
- Varying support for `tools`, `tool_choice`, `vision`, `parallel_tool_calls`
- Some fields are accepted but silently ignored

Therefore, instead of repurposing `aigw-openai` as a "third-party compatibility bus," we maintain a separate configurable compat layer.

## Responsibility Boundaries

- `aigw-openai`
  - Covers only the official OpenAI standard
  - Goal: maximum protocol fidelity
- `aigw-openai-compat`
  - Covers the OpenAI-compatible ecosystem
  - Uses `quirks` to describe per-provider differences
  - Promises "OpenAI-shaped," not "OpenAI-identical"

## Current Skeleton

The minimal configuration model so far:

- `OpenAICompatProvider`
- `OpenAICompatConfig`
- `Quirks`
- `OpenAICompatConfigError`

Roadmap suggestions:

1. Extract a shared HTTP/SSE transport layer for `aigw-openai` and `aigw-openai-compat` to share.
2. The compat layer should prioritize support for:
   - `chat/completions`
   - `embeddings`
   - Common streaming chat SSE
3. `Responses API` support should be gated behind capability detection or provider presets — do not assume it exists by default.
4. All provider-specific differences should be expressed through `quirks` or a capability matrix. Do not hardcode Groq/Together/vLLM branches into the official OpenAI crate.
