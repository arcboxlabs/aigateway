# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test

```bash
cargo check --workspace          # Fast type-check
cargo build --workspace          # Debug build
cargo test --workspace           # All tests
cargo test -p aigw-anthropic     # Single crate
cargo test -- --nocapture        # With stdout
```

## Architecture

Rust 2024 workspace. Four provider crates under `providers/`, each a library crate. Root `aigateway` crate will be the gateway binary.

### Provider Crates

- **`aigw-openai`** -- Official OpenAI client. Wire types match the OpenAI spec exactly. Supports both Responses API and Chat Completions. `#![forbid(unsafe_code)]`.
- **`aigw-openai-compat`** -- Configurable layer for OpenAI-compatible third-party APIs (Groq, Together, vLLM, Fireworks, Perplexity, LM Studio, etc.). Uses `Quirks` struct to declare per-provider capability flags. Shares transport/types from `aigw-openai`.
- **`aigw-anthropic`** -- Full protocol translation for Anthropic Messages API. Native types, SSE streaming with named events, `SecretString` for API keys, `bon` derive for builders.
- **`aigw-gemini`** -- Google Gemini provider (skeleton).

### Decision: new provider crate or compat config?

If the provider uses OpenAI-compatible API format -> add a `Quirks` config to `aigw-openai-compat`. Only create a new crate for providers with a distinct wire format (like Anthropic, Gemini).

### Key Patterns

**Wire-type fidelity**: Each provider's types mirror the actual API spec. No "lowest common denominator" types in protocol layer. Translation happens at the gateway layer via `TryFrom`/`Into`.

**Forward compatibility**: All request/response structs use `#[serde(flatten)] extra: JsonObject` to preserve unknown fields. Enums use `#[serde(other)] Unknown` or `Typed(T) | Raw(JsonObject)` untagged pattern for new variants.

**Content duality**: Both OpenAI and Anthropic allow content as string or array of blocks. Modeled as `#[serde(untagged)] enum { Text(String), Parts(Vec<ContentPart>) }`.

**Streaming**: `eventsource-stream` for SSE parsing. Anthropic requires a state machine to translate block-level events (`content_block_start/delta/stop`) into OpenAI-style choice-level deltas. Track `tool_call_index` and message metadata across events.

**Secrets**: API keys use `secrecy::SecretString`. Never derive `Debug` on config structs that hold raw keys -- use manual Debug impl or SecretString.

**Builders**: Use `bon::Builder` derive instead of hand-written `new()`/`with_*()`. Apply `#[builder(on(String, into))]` at struct level, `#[builder(default)]` for optional non-Option fields.

**Config normalization**: Config structs implement `normalize()` called at construction time -- trims whitespace, strips trailing slashes, validates URLs, returns `Result`.

## Docs

- `docs/provider-translation.md` -- Field-level mapping tables for OpenAI <-> Anthropic <-> Gemini translation
- `docs/anthropic-api-spec.md` -- Anthropic Messages API spec (request/response/streaming events)
- `docs/best-practices.md` -- Type modeling, error handling, streaming, crate architecture rationale
- `providers/aigw-openai/README.md` -- OpenAI API strategy, Responses API vs Chat Completions positioning
