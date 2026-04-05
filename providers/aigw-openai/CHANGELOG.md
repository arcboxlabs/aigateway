# Changelog

All notable changes to this project will be documented in this file.
## [0.2.0](https://github.com/arcboxlabs/aigateway/compare/aigw-openai-v0.1.0...aigw-openai-v0.2.0) - 2026-04-05

### Added
- *(aigw-core)* add canonical model, translator traits, and unify JsonObject- implement translate layer for openai, openai-compat, and anthropic providers

### Fixed
- resolve CI failures — cargo fmt and clippy collapsible_if

### Miscellaneous
- unify all crate versions via workspace.package

## [0.1.0](https://github.com/arcboxlabs/aigateway/compare/aigw-openai-v0.0.1...aigw-openai-v0.1.0) - 2026-04-05

### Added
- *(aigw-openai)* Responses API wire types, transport improvements

### Refactored
- *(aigw-openai)* replace hand-written constructors with bon builders

### Miscellaneous
- update repository org from AprilNEA to arcboxlabs- add GitHub Actions workflow with check, test, fmt, clippy, audit

### Init
- workspace with openai, openai-compat, anthropic, gemini provider crates
