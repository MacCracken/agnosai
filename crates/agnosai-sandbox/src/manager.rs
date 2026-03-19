//! SandboxManager — selects backend based on tool policy.
//!
//! Backends (in order of isolation strength):
//! 1. In-process (native Rust tools — no sandbox needed)
//! 2. WASM (wasmtime — memory isolation, capability-controlled)
//! 3. Process (subprocess + seccomp + Landlock + cgroups)
//! 4. OCI (container sandbox — strongest isolation)

// TODO: Implement sandbox manager
