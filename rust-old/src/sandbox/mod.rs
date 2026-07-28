//! Tool execution isolation — WASM, process, and OCI sandboxing.
//!
//! Untrusted tool code runs inside a sandbox. Two backends are implemented:
//!
//! - **WASM** (wasmtime) — memory-isolated, fuel-limited, no filesystem/network
//! - **Python subprocess** — stdin/stdout JSON protocol with timeout and kill-on-drop
//!
//! All subprocess-based sandboxes (Python, Process) strip dangerous environment
//! variables before spawning child processes. See [`SANITIZED_ENV_VARS`].

/// Environment variables removed from child processes to prevent library injection.
///
/// These are stripped from every subprocess spawned by the Python and Process sandboxes.
pub const SANITIZED_ENV_VARS: &[&str] = &[
    "LD_PRELOAD",
    "LD_LIBRARY_PATH",
    "DYLD_INSERT_LIBRARIES",
    "DYLD_LIBRARY_PATH",
];

#[cfg(feature = "kavach")]
pub mod kavach_bridge;
pub mod manager;
pub mod oci;
pub mod policy;
pub mod process;
pub mod python;
pub mod wasm;

pub use manager::{SandboxManager, SandboxManagerConfig, SandboxResult};
pub use oci::{OciSandbox, OciSandboxConfig};
pub use policy::{IsolationLevel, SandboxPolicy};
pub use process::{ProcessResult, ProcessSandbox, ProcessSandboxConfig};
pub use python::PythonSandbox;
pub use wasm::{WasmModule, WasmResult, WasmSandbox};
