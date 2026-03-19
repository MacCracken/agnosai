//! Sandboxed Python interpreter for legacy tools.
//!
//! Protocol: stdin JSON → python3 → stdout JSON
//! Sandbox: seccomp + Landlock + cgroups + network namespace

// TODO: Implement Python sandbox bridge
