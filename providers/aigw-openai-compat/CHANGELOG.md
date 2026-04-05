# Changelog

All notable changes to this project will be documented in this file.
## [0.2.0](https://github.com/arcboxlabs/aigateway/compare/aigw-openai-compat-v0.0.1...aigw-openai-compat-v0.2.0) - 2026-04-05

### Added
- implement translate layer for openai, openai-compat, and anthropic providers

### Fixed
- resolve CI failures — cargo fmt and clippy collapsible_if

### Miscellaneous
- unify all crate versions via workspace.package

## [0.0.1](https://github.com/arcboxlabs/aigateway/releases/tag/aigw-openai-compat-v0.0.1) - 2026-04-05

### Added
- *(aigw-openai)* Responses API wire types, transport improvements

### Documentation
- *(aigw-openai-compat)* translate README from Chinese to English

### Miscellaneous
- update repository org from AprilNEA to arcboxlabs- add GitHub Actions workflow with check, test, fmt, clippy, audit

### Init
- workspace with openai, openai-compat, anthropic, gemini provider crates
