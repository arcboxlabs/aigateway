# Changelog

All notable changes to this project will be documented in this file.
## [0.4.0](https://github.com/arcboxlabs/aigateway/compare/v0.3.0...v0.4.0) - 2026-04-15

### Added
- *(aigw-openai)* expose build_responses_create_request for library consumers

## [0.3.0](https://github.com/arcboxlabs/aigateway/compare/v0.2.0...v0.3.0) - 2026-04-15

### Added
- *(aigw-openai)* add Responses API translation layer with Codex support

### Documentation
- add Responses API translation reference and update design notes

### Miscellaneous
- *(release-plz)* disable semver_check during 0.x rapid iteration

### Style
- apply nightly cargo fmt to new responses translation modules

## [0.2.0](https://github.com/arcboxlabs/aigateway/compare/v0.1.0...v0.2.0) - 2026-04-05

### Added
- *(aigw-core)* add canonical model, translator traits, and unify JsonObject- implement translate layer for openai, openai-compat, and anthropic providers

### Fixed
- resolve CI failures — cargo fmt and clippy collapsible_if

### Refactored
- Gemini forward-compat extra fields, normalize validation, umbrella crate aliases

### Miscellaneous
- unify all crate versions via workspace.package

## [0.1.0](https://github.com/arcboxlabs/aigateway/compare/v0.0.1...v0.1.0) - 2026-04-05

### Added
- *(aigw)* add umbrella crate re-exporting all providers- *(aigw-openai)* Responses API wire types, transport improvements- *(aigw-anthropic)* transport layer, rate limits, split types modules- *(aigw-gemini)* scaffold Gemini provider crate

### Refactored
- *(anthropic)* replace hand-written builders with bon derive- *(aigw-openai)* replace hand-written constructors with bon builders- *(aigw-anthropic)* review fixes, SecretString, bon builder, docs

### Documentation
- add README with architecture diagram, provider logos, and CONTRIBUTING guide- *(aigw-openai-compat)* translate README from Chinese to English

### Miscellaneous
- add release-plz workflow and config for automated releases- update repository org from AprilNEA to arcboxlabs- add GitHub Actions workflow with check, test, fmt, clippy, audit

### Init
- workspace with openai, openai-compat, anthropic, gemini provider crates

### Style
- apply cargo fmt to aigw-gemini, update lockfile
