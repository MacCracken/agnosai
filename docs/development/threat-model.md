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
| Zombie processes | Every spawned child is waited on; the sandbox tiers enforce a SIGKILL deadline. ⚠ There is no tokio and no `kill_on_drop` in this tree |
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
| Type confusion | ⚠ Cyrius has no `#[non_exhaustive]` and far weaker typing than Rust — most values are `i64` handles. The mitigation here is the deserialisers REJECTING a record that is missing a required field rather than defaulting it, plus explicit tag checks on every wire enum |
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

## Controls added 2026-08-13

Two adversarial reviews of the port fixed 27 defects; these are the ones that
change the security posture, and each is mutation-verified.

| control | what it stops |
|---|---|
| **Rate limiting mounted by default** (100 req/s per key, burst 200) | an unauthenticated flood. ⚠ An invalid bearer token still costs a full RSA verify (1.202 ms), because the `alg` check deliberately sits after signature verification. [ADR 021](../adr/021-rate-limit-mounted-by-default.md) |
| **Client key capped at 64 bytes** | a 10 MiB `X-Forwarded-For` becoming a permanent per-key allocation; above 4 KiB the freelist mmaps a VMA per key that is never unmapped |
| **Bucket eviction actually runs**, with a pressure rung | unbounded map growth on an attacker-chosen key. Idle eviction alone cannot bound a client presenting a fresh key per request |
| **Probes exempt from limiting** | a `GET /health` flood draining the shared `"unknown"` bucket and crash-looping the pod via its liveness probe |
| **WASM fuel AND memory bounds reach wasmtime** | a guest burning unbounded CPU (was 30x the intended budget) or claiming up to 4 GiB (was unbounded on the default policy). kavach 3.11.11 / 3.11.12 |
| **`concurrent_users: 0` guarded at the divide** | an unauthenticated `POST /mcp` raising SIGFPE and killing the whole process |
| **Strict env parsing** | `AGNOSAI_RATE_LIMIT=<garbage>` silently disabling the limiter, or a value above 65535 silently clamping to 100 req/s |

⚠ **Known and NOT mitigated: `X-Forwarded-For` spoofing cuts both ways.** The
limiter runs before auth and takes the header's first entry verbatim, so an
unauthenticated client can drain a **named victim's** bucket. "Run behind a
proxy" does not fix it on nginx's documented `$proxy_add_x_forwarded_for`, which
**appends** — only a proxy that OVERWRITES the header makes the key trustworthy.

## Accepted Risks

- **TOCTOU on tool paths**: Time-of-check-time-of-use gap between resolving a
  tool binary path and executing it. Mitigated by running in controlled
  environments with trusted `$PATH`.
- **Python/process subprocess escapes**: ⚠ **NOT confined.** seccomp-bpf, Landlock
  and cgroups are named by the oracle's module doc as "(future)" and are not
  yet implemented. Current mitigation: a SIGKILL deadline, a controlled working
  directory and a sanitized environment — no kernel confinement. Treat a
  process-tier tool as running with the server's privileges.
- **Fleet coordinator election**: Simple "first node wins" election. No Byzantine
  fault tolerance. Suitable for trusted internal networks.
