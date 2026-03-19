# Changelog

All notable changes to AgnosAI will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Workspace scaffold with 9 crates
- `agnosai-core`: Core types — Agent, Task, Crew, Message, Resource, Error
- `agnosai-orchestrator`: Orchestrator with `Arc<RwLock<State>>`, priority scheduler, scoring stub
- `agnosai-llm`: LlmProvider trait, InferenceRequest/Response types, 9 provider stubs
- `agnosai-fleet`: Module stubs for registry, placement, relay, coordinator, state, GPU, federation
- `agnosai-sandbox`: Module stubs for WASM, process, OCI, Python sandboxing
- `agnosai-tools`: Module stubs for tool registry, native/WASM/Python execution
- `agnosai-learning`: Module stubs for profiling, UCB1, replay buffer, capability scoring, RL optimizer
- `agnosai-definitions`: Module stubs for loader, assembler, versioning, packaging
- `agnosai-server`: Minimal axum server with health/ready endpoints
- `simple_crew` example
