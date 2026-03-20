//! AgnosAI — provider-agnostic AI orchestration framework.
//!
//! This crate unifies core types, LLM providers, task orchestration, tool
//! execution, adaptive learning, and an HTTP API server into a single library.

#![warn(missing_docs)]

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
