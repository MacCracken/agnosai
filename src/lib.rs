//! AgnosAI — provider-agnostic AI orchestration framework.
//!
//! This crate unifies core types, LLM providers, task orchestration, tool
//! execution, adaptive learning, and an HTTP API server into a single library.
//!
//! # Feature flags
//!
//! All features are opt-in (no defaults):
//!
//! - **`sandbox`** — WASM (wasmtime) and Python subprocess tool sandboxing.
//! - **`fleet`** — Distributed multi-node crew execution and GPU scheduling.
//! - **`definitions`** — YAML/JSON agent preset loading and `.agpkg` packaging.
//! - **`hwaccel`** — Hardware accelerator detection via [`ai-hwaccel`](https://github.com/maccracken/ai-hwaccel).
//! - **`full`** — Enables all of the above.

pub mod core;
pub mod learning;
pub mod llm;
pub mod orchestrator;
pub mod server;
pub mod tools;

#[cfg(feature = "sandbox")]
pub mod sandbox;

#[cfg(feature = "fleet")]
pub mod fleet;

#[cfg(feature = "definitions")]
pub mod definitions;
