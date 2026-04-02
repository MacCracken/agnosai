# Threat Model

AgnosAI orchestrates AI agent crews, executes tools in sandboxed environments,
communicates with LLM providers via HTTP, and optionally distributes work across
a fleet of nodes. This document maps the attack surfaces and mitigations.

## Attack Surfaces

### 1. HTTP API (axum server)

Clients send JSON to create crews, submit agent definitions, and stream results.

| Threat | Mitigation |
|--------|------------|
| Oversized request body | `DefaultBodyLimit::max(10 MiB)` on all routes |
| Malformed JSON / extra fields | `#[serde(deny_unknown_fields)]` on API request types |
| Missing/invalid auth | JWT RS256 validation with issuer + audience + expiry checks |
| Injection via agent definitions | Input validation (`MAX_AGENTS`, `MAX_TASKS`, `MAX_STRING_LEN`) |
| Slow-loris / connection exhaustion | axum + hyper connection limits; deploy behind reverse proxy |

### 2. Tool Execution (WASM, Python, Process)

Tools run untrusted code from community authors or legacy Python scripts.

| Threat | Mitigation |
|--------|------------|
| Memory exhaustion | WASM: 64 MiB `StoreLimits`; Process: `setrlimit` (best-effort) |
| CPU exhaustion | WASM: 1B fuel budget + 30s epoch timeout |
| Filesystem access | WASM: no filesystem capability; Process: optional `clean_env` |
| Network access | WASM: no network capability; Process: namespace isolation (future) |
| Library injection (`LD_PRELOAD`) | All subprocesses strip `LD_PRELOAD`, `LD_LIBRARY_PATH`, `DYLD_INSERT_LIBRARIES`, `DYLD_LIBRARY_PATH` |
| Zombie processes | `kill_on_drop(true)` on all tokio child processes |
| Output flooding | WASM: 1 MiB stdout buffer; Process: `wait_with_output` bounded by timeout |

### 3. LLM Provider Communication

HTTP requests to OpenAI, Anthropic, Ollama, and other providers.

| Threat | Mitigation |
|--------|------------|
| API key leakage | Keys in environment variables, never logged or serialized |
| Provider impersonation | TLS via `rustls` with certificate validation (no `danger_accept_invalid_certs` in production) |
| Response injection | LLM responses treated as untrusted data; never executed as code |
| Cost runaway | `ResourceBudget` with `max_tokens`, `max_cost_usd`, `max_duration_secs` |

### 4. Fleet Communication

When the `fleet` feature is enabled, nodes exchange placement plans and results.

| Threat | Mitigation |
|--------|------------|
| Unauthenticated nodes | Fleet endpoints behind same auth middleware as API |
| Message tampering | Deploy behind authenticated transport (mTLS, VPN) in production |
| Node impersonation | Node IDs are self-assigned; use network-level auth for trust |
| State corruption | `Arc<RwLock>` with scoped write locks; no shared mutable state across nodes |

### 5. Serialization / Deserialization

Agent definitions, crew specs, and presets are loaded from JSON/YAML.

| Threat | Mitigation |
|--------|------------|
| Unknown fields in JSON | `#[serde(deny_unknown_fields)]` on API input types |
| Type confusion | Strong Rust typing; `#[non_exhaustive]` on all public enums |
| Oversized payloads | Request body limits; validation bounds on string lengths and collection sizes |
| Malicious YAML | YAML parsing only in `definitions` feature; `serde_yaml_ng` with default limits |

### 6. Supply Chain

Dependencies could contain malicious code.

| Threat | Mitigation |
|--------|------------|
| Vulnerable dependencies | `cargo-audit` in CI |
| License violations | `cargo-deny` with explicit license allowlist |
| Dependency auditing | `cargo-vet` with Mozilla imports |
| Typosquatting | `deny.toml` restricts to crates.io registry only |
| Wildcard versions | `cargo-deny` denies wildcard version specs |

## Trust Boundaries

```
Untrusted                    Trusted
─────────────────────────────────────────
HTTP clients          →  API validation layer  →  Orchestrator
WASM tool modules     →  Sandbox (wasmtime)    →  Tool registry
Python tool scripts   →  Subprocess sandbox    →  Tool registry
LLM provider responses →  Response parsing     →  Agent execution
JSON/YAML definitions →  Serde validation      →  Definition store
Fleet messages        →  Auth + TLS            →  Coordinator
```

## Accepted Risks

- **TOCTOU on tool paths**: Time-of-check-time-of-use gap between resolving a
  tool binary path and executing it. Mitigated by running in controlled
  environments with trusted `$PATH`.
- **Python subprocess escapes**: seccomp-bpf and Landlock are planned but not
  yet implemented. Current mitigation: timeout + kill-on-drop + env sanitization.
- **Fleet coordinator election**: Simple "first node wins" election. No Byzantine
  fault tolerance. Suitable for trusted internal networks.
