# Changelog

All notable changes to this project will be documented in this file.
## [0.2.0](https://github.com/arcboxlabs/aigateway/compare/aigw-gemini-v0.1.0...aigw-gemini-v0.2.0) - 2026-04-05

### Fixed
- resolve CI failures — cargo fmt and clippy collapsible_if

### Refactored
- Gemini forward-compat extra fields, normalize validation, umbrella crate aliases

### Miscellaneous
- unify all crate versions via workspace.package

## [0.1.0](https://github.com/arcboxlabs/aigateway/compare/aigw-gemini-v0.0.1...aigw-gemini-v0.1.0) - 2026-04-05

### Added
- *(aigw-gemini)* scaffold Gemini provider crate

### Miscellaneous
- update repository org from AprilNEA to arcboxlabs

### Init
- workspace with openai, openai-compat, anthropic, gemini provider crates

### Style
- apply cargo fmt to aigw-gemini, update lockfile
