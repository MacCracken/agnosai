//! AgnosAI — provider-agnostic AI orchestration framework.
//!
//! This crate unifies core types, LLM providers, task orchestration, tool
//! execution, adaptive learning, and an HTTP API server into a single library.

pub mod core;
pub mod llm;
pub mod orchestrator;
pub mod tools;
pub mod learning;
pub mod server;

#[cfg(feature = "sandbox")]
pub mod sandbox;

#[cfg(feature = "fleet")]
pub mod fleet;

#[cfg(feature = "definitions")]
pub mod definitions;
