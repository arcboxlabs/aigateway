# Contributing

Thank you for your interest in contributing to AI Gateway! This guide covers everything you need to get started.

## Prerequisites

- **Rust** (2024 edition — stable 1.85+ or nightly)
- **Cargo** (comes with Rust)

## Build & Test

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

## Project Structure

```
aigateway/
├── src/                          # Gateway binary
│   └── main.rs
├── providers/
│   ├── aigw-openai/              # Official OpenAI provider
│   ├── aigw-openai-compat/       # OpenAI-compatible third parties
│   ├── aigw-anthropic/           # Anthropic Messages API
│   └── aigw-gemini/              # Google Gemini (skeleton)
├── docs/
│   ├── provider-translation.md   # Field-level mapping tables
│   ├── anthropic-api-spec.md     # Anthropic API reference
│   └── best-practices.md         # Type modeling & architecture rationale
└── Cargo.toml                    # Workspace root
```

## Code Conventions

### Serde & Wire Types

Wire types mirror the upstream API exactly. Every struct should carry a catch-all for forward compatibility:

```rust
#[serde(flatten)]
pub extra: JsonObject,
```

Enums should use `#[serde(other)] Unknown` or a `Typed(T) | Raw(JsonObject)` pattern.

### Builders

Use `bon::Builder` derive — no hand-written builder methods:

```rust
#[derive(bon::Builder)]
#[builder(on(String, into))]
pub struct FooRequest { ... }
```

### Secrets

API keys **must** be `secrecy::SecretString`. Never derive `Debug` on raw key fields.

### Errors

Use `thiserror`. Preserve `#[source]`/`#[from]` chains. Separate variants for HTTP / JSON / API / Stream / Config errors. Non-JSON error responses should use `UnexpectedResponse { status, body }`.

### Streaming

Use `eventsource-stream` for SSE. Dispatch on the JSON `type` field, not the SSE `event:` line. Anthropic block-level events require a state machine to translate to choice-level deltas.

### Imports

Group imports with blank lines between groups:

```rust
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::types::JsonObject;
```

### Naming

- `snake_case` modules
- `PascalCase` types matching upstream API names (e.g. `MessagesRequest`, `ContentBlock`)

## Adding a New Provider

### New crate or compat config?

| Scenario | Action |
|----------|--------|
| Provider uses OpenAI-compatible API format | Add a `Quirks` config to `aigw-openai-compat` |
| Provider has a distinct wire format | Create a new crate under `providers/` |

### New crate checklist

1. Create `providers/aigw-{name}/` with `Cargo.toml` and `src/lib.rs`
2. Add the crate to the workspace `members` in the root `Cargo.toml`
3. Model the provider's native wire types — no "universal" abstractions
4. Implement SSE streaming with full event fidelity
5. Use `SecretString` for API keys, `thiserror` for errors
6. Add tests (snapshot tests and wire-format round-trip tests are preferred)

## Documentation

| Document | Purpose |
|----------|---------|
| [`docs/provider-translation.md`](docs/provider-translation.md) | Field-level mapping tables for OpenAI ↔ Anthropic ↔ Gemini |
| [`docs/anthropic-api-spec.md`](docs/anthropic-api-spec.md) | Anthropic Messages API spec |
| [`docs/best-practices.md`](docs/best-practices.md) | Type modeling, error handling, streaming, crate architecture rationale |
