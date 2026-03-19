//! Tool execution isolation — WASM, process, and OCI sandboxing.
//!
//! Untrusted tool code runs inside a sandbox. Two backends are implemented:
//!
//! - **WASM** (wasmtime) — memory-isolated, fuel-limited, no filesystem/network
//! - **Python subprocess** — stdin/stdout JSON protocol with timeout and kill-on-drop

pub mod manager;
pub mod oci;
pub mod policy;
pub mod process;
pub mod python;
pub mod wasm;
