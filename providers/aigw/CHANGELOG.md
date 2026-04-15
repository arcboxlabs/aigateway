# Changelog

All notable changes to this project will be documented in this file.
## [0.4.0](https://github.com/arcboxlabs/aigateway/compare/aigw-v0.3.0...aigw-v0.4.0) - 2026-04-15

### Added
- *(aigw-openai)* expose build_responses_create_request for library consumers

## [0.3.0](https://github.com/arcboxlabs/aigateway/compare/aigw-v0.2.0...aigw-v0.3.0) - 2026-04-15

### Added
- *(aigw-openai)* add Responses API translation layer with Codex support

### Style
- apply nightly cargo fmt to new responses translation modules

## [0.2.0](https://github.com/arcboxlabs/aigateway/compare/aigw-v0.0.1...aigw-v0.2.0) - 2026-04-05

### Added
- *(aigw-core)* add canonical model, translator traits, and unify JsonObject- implement translate layer for openai, openai-compat, and anthropic providers

### Fixed
- resolve CI failures — cargo fmt and clippy collapsible_if

### Refactored
- Gemini forward-compat extra fields, normalize validation, umbrella crate aliases

### Miscellaneous
- unify all crate versions via workspace.package

## [0.0.1](https://github.com/arcboxlabs/aigateway/releases/tag/aigw-v0.0.1) - 2026-04-05

### Added
- *(aigw)* add umbrella crate re-exporting all providers
