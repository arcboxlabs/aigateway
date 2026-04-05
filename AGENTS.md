# AGENTS.md

## Commands
```bash
cargo check --workspace              # Fast type-check
cargo build --workspace              # Debug build
cargo test --workspace               # All tests
cargo test -p aigw-anthropic         # Single crate
cargo test -p aigw-openai -- chat    # Single test by name substring
cargo test --doc -p aigw-anthropic   # Doc tests only
cargo test -- --nocapture            # With stdout
cargo clippy --workspace             # Lint
```

## Architecture
Rust 2024 workspace. Four provider crates under `providers/`, each a library crate. Root `aigateway` crate will be the gateway binary.

### Provider Crates
- **`aigw-openai`** — Official OpenAI client. Wire types match the OpenAI spec exactly. Supports both Responses API and Chat Completions. `#![forbid(unsafe_code)]`.
- **`aigw-openai-compat`** — Configurable layer for OpenAI-compatible third-party APIs (Groq, Together, vLLM, Fireworks, Perplexity, LM Studio, etc.). Uses `Quirks` struct to declare per-provider capability flags. Shares transport/types from `aigw-openai`.
- **`aigw-anthropic`** — Full protocol translation for Anthropic Messages API. Native types, SSE streaming with named events, `SecretString` for API keys, `bon` derive for builders.
- **`aigw-gemini`** — Google Gemini provider (skeleton).

### Decision: new provider crate or compat config?
If the provider uses OpenAI-compatible API format → add a `Quirks` config to `aigw-openai-compat`. Only create a new crate for providers with a distinct wire format (like Anthropic, Gemini).

## Code Style
- **Serde**: Wire types mirror upstream API exactly. Use `#[serde(flatten)] extra: JsonObject` on structs, `#[serde(other)] Unknown` or `Typed(T) | Raw(JsonObject)` on enums for forward compat.
- **Builders**: `bon::Builder` derive with `#[builder(on(String, into))]`. No hand-written builder methods. `#[builder(default)]` for optional non-Option fields.
- **Secrets**: API keys must be `secrecy::SecretString`. Never derive `Debug` on raw key fields.
- **Errors**: Use `thiserror`. Preserve `#[source]`/`#[from]` chains. Separate variants for HTTP/JSON/API/Stream/Config. Non-JSON error responses → `UnexpectedResponse { status, body }`.
- **Streaming**: `eventsource-stream` for SSE. Dispatch on JSON `type` field, not SSE `event:` line. Anthropic block-level events need a state machine to translate to choice-level deltas.
- **Content duality**: String-or-array modeled as `#[serde(untagged)] enum { Text(String), Parts(Vec<ContentPart>) }`.
- **Config normalization**: Config structs implement `normalize()` at construction — trims whitespace, strips trailing slashes, validates URLs, returns `Result`.
- **Imports**: Group std → external crates → `crate::` with blank lines. Prefer specific imports over globs.
- **Naming**: snake_case modules, PascalCase types matching upstream API names (e.g. `MessagesRequest`, `ContentBlock`).
- **Translation**: No "lowest common denominator" types in protocol layer. Translation happens at the gateway layer via `TryFrom`/`Into`.

## Docs
- `docs/provider-translation.md` — Field-level mapping tables for OpenAI ↔ Anthropic ↔ Gemini translation
- `docs/anthropic-api-spec.md` — Anthropic Messages API spec (request/response/streaming events)
- `docs/best-practices.md` — Type modeling, error handling, streaming, crate architecture rationale
- `providers/aigw-openai/README.md` — OpenAI API strategy, Responses API vs Chat Completions positioning
