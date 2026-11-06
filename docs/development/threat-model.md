# Threat Model

AgnosAI orchestrates AI agent crews, executes tools in sandboxed environments,
communicates with LLM providers via HTTP, and optionally distributes work across
a fleet of nodes. This document maps the attack surfaces and mitigations.

> **Refreshed at 2.2.3 against the Cyrius tree.** Until then, surfaces 1–6 still described
> the Rust build: axum, serde, rustls, `Arc<RwLock>`, cargo-audit. Several of those rows
> claimed mitigations this tree does not have, and each is now marked ⚠. The Rust-era text
> is `git show 2.2.2:docs/development/threat-model.md`. Cyrius security advisories are cited
> by their `CYRIUS-2026-NNNN` ids.

## Attack Surfaces

### 1. HTTP API (sandhi's pooled server)

Clients send JSON to create crews, submit agent definitions, and stream results.

| Threat | Mitigation |
|--------|------------|
| Oversized request body | `AGNOSAI_MAX_BODY_BYTES` (10 MiB, `src/server/router.cyr`), enforced by sandhi's `max_request` before the handler runs |
| Malformed JSON / extra fields | bayan's parser refuses malformed JSON, a duplicate key and a leading-zero number (since bayan 1.11.1, CHANGELOG 2.2.1), and nesting past depth 128. Request types reject unknown fields with `agnosai_route_no_unknown_fields` |
| Missing/invalid auth | JWT RS256 (2048–4096-bit keys), checking signature, then `alg`, `iss`, `aud` and `exp` (`src/server/auth.cyr`). ⚠ **Auth is off by default** (`agnosai_auth_config_new`), and the server warns at boot |
| Injection via agent definitions | Input validation (`MAX_AGENTS`, `MAX_TASKS`, `MAX_STRING_LEN`); user-supplied fields are wrapped before they reach a prompt |
| Slow-loris / connection exhaustion | 100 pool workers (`AGNOSAI_SERVE_WORKERS`) as the connection cap, and sandhi's 60 s accept-to-request budget (`SANDHI_SERVER_DEFAULT_REQUEST_MS`, sandhi 1.10.7), after which the request is answered 408. Rate limiting is on by default (below). An SSE stream holds a worker for a crew's life (roadmap B8) |

### 2. Tool Execution (WASM, cx, Python, Process, OCI)

Tools run untrusted code from community authors or legacy Python scripts.

| Threat | Mitigation |
|--------|------------|
| Memory exhaustion | WASM: kavach passes `-W max-memory-size` (64 MiB default) to wasmtime. ⚠ **Process / Python / OCI: no memory limit.** There is no `setrlimit` here or in the Rust line (`src/sandbox/process.cyr`'s header) |
| CPU exhaustion | WASM: 1B fuel (`-W fuel`) plus the sandbox deadline; every tier has a SIGKILL deadline |
| Filesystem access | WASM: no filesystem capability; cx: landlock read-only on the interpreter's directory; ⚠ Process / Python / OCI: a controlled working directory only (see *Accepted Risks*) |
| Network access | WASM: no network capability; cx: a new network namespace, and the cx tier **fails closed** where one cannot be made (CHANGELOG 2.2.1); ⚠ Process / Python / OCI: none |
| A model steered to a host its task did not declare (the system under test writes tool output, so a page can ask for it) | A consumer's gate ([ADR 025](../adr/025-a-tool-call-passes-a-consumers-gate.md)), asked at `agnosai_tool_execute_for` and of each URL, so no tool implements it and no path can skip it. ⚠ With no gate, a tool's target is still the model's to choose, under the SSRF guard alone |
| A redirect off the checked target | Every hop goes back through `agnosai_tool_url_ok` before its request is made (`agnosai_guarded_fetch_hops`), so the check does not end at the first 302. `load_testing` follows no redirect |
| A gate's admission reaching cloud credentials | An ADMIT gets a declared host past `agnosai_is_safe_url` and no further: 169.254/16, `fd00:ec2::254`, `metadata.google.internal`, `metadata.goog` and non-HTTP schemes are refused whatever the gate says (`agnosai_url_is_admissible`), and any verdict but PASS or ADMIT refuses. ⚠ Neither check resolves names, so a declared name that resolves somewhere it should not is the gate's to answer for |
| A private address spelled so the guard and the dialer read it differently | The guard judges a host by `sandhi_net_parse_ipv4`, the parser that dials, as well as the WHATWG reading, and a host must pass under both. Until 2.2.4 `0127.0.0.1` read as public octal and connected to loopback (CHANGELOG 2.2.4 *Security*). A name is compared with its root dots dropped, as a resolver reads it, so `foo.internal.` is `foo.internal` |
| Library injection (`LD_PRELOAD`) | All subprocesses strip `LD_PRELOAD`, `LD_LIBRARY_PATH`, `DYLD_INSERT_LIBRARIES`, `DYLD_LIBRARY_PATH` (`agnosai_sanitized_envp`) |
| Zombie processes | Every spawned child is waited on; the sandbox tiers enforce a SIGKILL deadline |
| Output flooding | 1 MiB stdout cap on the kavach paths; the spawner's capture is bounded by its deadline |
| ⚠ **A guest that hangs its worker** (found 2.2.2) | **Not mitigated.** kavach's capture writes a WASM guest's whole stdin before draining its stdout, so a guest that writes while it reads deadlocks once its input passes about two pipe buffers (~100–200 KB), and kavach's deadline never fires. Roadmap B9 / B16 |

### 3. LLM Provider Communication

HTTP requests to hoosh (the gateway), and through it to providers.

| Threat | Mitigation |
|--------|------------|
| API key leakage | Keys in environment variables, never logged or serialized |
| The gateway's URL in a result | A transport error names its failure kind and nothing else, since that text becomes a task's `output`, which a consumer serves and stores. A log line says which gateway from sandhi's parsed scheme, host and port, and withholds a URL holding an `@` or an odd host (`agnosai_hoosh_log_url_a`), at start-up too |
| Provider impersonation | The stdlib's native TLS with chain and hostname verification, nameConstraints enforced since cyrius 6.7.18 (CYRIUS-2026-0042), and extendedKeyUsage checked at every depth (CYRIUS-2026-0026) |
| Response injection | LLM responses treated as untrusted data; never executed as code |
| ⚠ Cost runaway | **Only `max_duration_secs` is enforced** (`src/orchestrator/orchestrator.cyr`). `ResourceBudget`'s token and cost limits are carried and not checked; enforcing them is roadmap F5 |

### 4. Fleet Communication

The fleet modules (`src/fleet/`) are **library API with no route**. agnosai's server exposes no
fleet endpoint, so a consumer that runs a fleet brings its own transport.

| Threat | Mitigation |
|--------|------------|
| Unauthenticated nodes, message tampering | The consumer's transport. Run it authenticated (mTLS, VPN); agnosai adds no fleet auth of its own |
| Node impersonation | Node IDs are self-assigned; use network-level auth for trust |
| A peer advancing the coordinator term | `agnosai_federation_declare_coordinator` adopts any declaration whose term is not stale: there is no vote or quorum, so any peer can name a coordinator at a higher term. Federation trusts its peers by design (`src/fleet/federation.cyr`, checked at 2.2.2); keep them on an authenticated transport |
| ⚠ State corruption | **No fleet type takes a lock** (`src/fleet/*.cyr`). A consumer that shares one across threads must serialise access itself |

### 5. Serialization / Deserialization

Agent definitions, crew specs, and presets are loaded from JSON/YAML.

| Threat | Mitigation |
|--------|------------|
| Unknown fields in JSON | `agnosai_route_no_unknown_fields` on API input types |
| Duplicate keys (one parser reading the first, another the last) | Refused by bayan 1.11.1, so a body or a JWT that repeats a key is 400 / 401 (CHANGELOG 2.2.1) |
| Type confusion | ⚠ Cyrius has no `#[non_exhaustive]` and far weaker typing than Rust — most values are `i64` handles. The mitigation here is the deserialisers REJECTING a record that is missing a required field rather than defaulting it, plus explicit tag checks on every wire enum |
| Oversized payloads | Request body limits; validation bounds on string lengths and collection sizes; JSON nesting capped at depth 128 |
| Malicious YAML | Only definition *files* are YAML (`src/definitions/loader.cyr`). bayan's subset parser refuses anchors and aliases (so no alias expansion), tags, block scalars and flow mappings |

### 6. Supply Chain

Dependencies could contain malicious code.

| Threat | Mitigation |
|--------|------------|
| A dependency changing under a tag | Every `[deps.*]` resolves to a commit pin in `cyrius.lock`; `deps --verify` (in `check-clean.sh`) checks `lib/` against the lock's hashes |
| A dependency downgrading a folded stdlib module | `check-clean.sh`'s lib-snapshot diff against the pinned toolchain's `lib/` |
| The toolchain itself | Pinned in `cyrius.cyml`; CI installs that exact version with a SHA256-checked tarball |
| CI actions | Every GitHub Action is pinned by commit SHA |
| Source-level policy | `cyrius vet` and `cyrius deny` in `check-clean.sh` |

### 7. Inbound trace context (W3C `traceparent`, [ADR 023](../adr/023-genai-semconv-spans-and-w3c-trace-context.md))

`POST /api/v1/crews`, `POST /api/v1/a2a/receive` and `/mcp` `tools/call` read
the `traceparent` header, and the work's spans join that trace. In-process,
`agnosai_crew_with_trace_parent` does the same for an embedding caller.

**Who can send it:** any client that reaches those routes. With auth on that is
any authenticated caller; with auth off — the default, which is fail-open
(`agnosai_auth_config_new`) — it is anyone. The header is honoured whenever the
OTLP exporter is on; nothing else gates it.

| Threat | Mitigation |
|--------|------------|
| Header as a parser attack (oversized, non-hex, uppercase, zero ids) | Strict W3C Trace Context Level 1 version `00` parse: exactly 55 bytes, `-` at fixed offsets, lowercase hex, non-zero trace and parent ids. Anything else starts a new root trace — never an error, never a different response. The spec keeps a 55-byte copy and nothing longer |
| Header changing what the client sees | The response does not depend on it: a missing, malformed or unsampled header answers the same status as a valid one, never a 4xx (`tests/server_serve.tcyr`) |
| **Trace-id choice** — the caller picks the trace id agnosai's spans join, so it can plant spans into an arbitrary trace on the collector, including a trace id it observed from another tenant | ⚠ **Not mitigated.** Joining the caller's trace is the feature. A collector shared across trust domains must not treat a trace id as proof of who produced a span |
| **Export suppression** — sampled flag `0` (`-00`) switches OFF span export for the work that caller starts (its crews, its tool calls), because sampling is ParentBased | ⚠ **Not mitigated.** A caller can hide its own work from OTLP tracing. Only spans are lost: sakshi logs, the audit chain, `/metrics` counters and the task results are unaffected. An operator switch to restart (or ignore) an inbound context is on the roadmap (B7, item 10) |

The W3C spec leaves to a receiving service whether an inbound trace crossing a
trust boundary is honoured or restarted. agnosai honours it unconditionally
today; that is a recorded choice, not an oversight.

## Trust Boundaries

```
Untrusted                    Trusted
─────────────────────────────────────────
HTTP clients          →  API validation layer  →  Orchestrator
traceparent header    →  strict W3C v00 parse  →  span parent + sampling (honoured as-is)
WASM tool modules     →  kavach + wasmtime     →  Tool registry
cx tool bytecode      →  kavach (ns, landlock) →  Tool registry
Python / process tools →  agnosai's spawner    →  Tool registry   (⚠ no kernel confinement)
LLM provider responses →  Response parsing     →  Agent execution
JSON/YAML definitions →  bayan + validators    →  Definition store
Fleet messages        →  the consumer's transport (agnosai serves no fleet endpoint)
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

- **TOCTOU on tool paths**: agnosai's own spawner (`src/sandbox/spawn.cyr`) resolves a tool
  binary against `PATH` and then execs it by path, so what runs is whatever sits at that path at
  exec time. kavach's paths pin the binary by fd before the fork (ADR-005 §H4 in kavach), but
  the process, Python and OCI tiers do not go through kavach (next item). Mitigated by running
  in controlled environments with a trusted `$PATH`.
- **Python/process/OCI subprocess escapes**: ⚠ **NOT confined.** These tiers spawn through
  agnosai's own spawner: a SIGKILL deadline, a controlled working directory and a sanitized
  environment, with no seccomp, landlock, namespaces or cgroup. Treat a process-tier tool as
  running with the server's privileges. **kavach's confined process backend is already bridged**
  (`agnosai_kavach_execute`: no_new_privs, landlock scopes and kavach's exec seccomp filter,
  tested by `tests/sandbox_kavach_bridge.tcyr`). But no production tier calls it; routing them
  through it is on the roadmap.
- **Fleet coordinator election**: lowest-id election, and any non-stale term is adopted (surface 4).
  No Byzantine fault tolerance. Suitable for trusted internal networks.
- **Caller-controlled trace context** (surface 7): any caller of a traced route
  chooses the trace its spans join and can suppress their export with sampled
  flag `0`. Accepted because joining the caller's trace is the point of the
  feature and nothing but OTLP spans is lost; the operator switch that would
  restart an untrusted inbound context is a roadmap item.
