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
| Missing/invalid auth | JWT RS256 (2048–4096-bit keys, in an SPKI, PKCS#1 or, since 2.6.1, certificate PEM, whose key alone is read: its dates and issuer are not checked, as the Rust line's were not), checking signature, then `alg`, `iss`, `aud` and `exp` (`src/server/auth.cyr`). ⚠ **Auth is off by default** (`agnosai_auth_config_new`), and the server warns at boot |
| Injection via agent definitions | Input validation (`MAX_AGENTS`, `MAX_TASKS`, `MAX_STRING_LEN`); user-supplied fields are wrapped before they reach a prompt |
| An unreviewed risky answer | Since 2.4.0 a `high`-risk task's answer waits for a reviewer on the orchestrator's gate, which `POST /api/v1/approvals` shares ([ADR 032](../adr/032-the-crew-lifecycle-is-one-state-machine.md)). Only a reviewer's decision approves: a timeout, a cancel, the crew's deadline, a full gate and a missing gate all reject, and a rejected answer is withheld from the crew's results and from any dependent task. Every decision is audited (`approval_decided`). Through 2.3.8 the gate existed and nothing waited on it |
| A client posting crews in a loop to grow the server's memory for good | Since 2.5.1 a crew the server builds lives in an arena its orchestrator gives back once its registry evicts it, past `AGNOSAI_MAX_RETAINED_CREWS` (1,000), and no route holds it ([ADR 037](../adr/037-the-servers-crews-are-the-orchestrators-to-give-back.md)): the REST loop reads 0 B a crew, where 2.5.0 kept 13,144. Since 2.5.2 so does the audit chain, which kept 6,360 B a crew past its 10,000-entry ring: an entry lives in a ring slot the chain resets when the ring takes it again ([ADR 038](../adr/038-an-audit-entry-lives-in-its-ring-slot.md)), so a full chain costs its ring, 11.9 MB, and no more |
| A request writing the A2A Agent Card's URL through its `Host` | Since 2.6.0 the card names its endpoint under `AGNOSAI_PUBLIC_URL` when set, and is then built once at start-up ([ADR 040](../adr/040-a2a-v1-tasks-are-crews.md)). Without it the card names the request's `Host`, accepted only as letters, digits, `.`, `-`, `:` and brackets, else `localhost`, and goes out `Cache-Control: no-cache`, so a host one client sends reaches that client's card and no shared cache. ⚠ Behind a proxy, set the public URL: the request's host is the proxy's view |
| A forged Agent Card | Since 2.6.0 the operator can sign the card (`AGNOSAI_A2A_SIGNING_KEY`, ES256 or RS256, a JWS over its RFC 8785 form), and `agnosai_a2a_card_verify_es256_a` / `_rs256_a` check one, as a client should before trusting it. Unsigned unless configured, as the spec allows |
| A2A tasks seen by another caller | `POST /a2a` takes the same auth as `/api/v1`. ⚠ **Tasks are scoped to that auth, not to a caller**: agnosai has one credential, and every authenticated caller can read, list and cancel every A2A task, as it can every crew through `/api/v1/crews` |
| An MCP gateway routing on headers the body contradicts | Since 2.6.0 a 2026-07-28 request's `MCP-Protocol-Version`, `Mcp-Method` and `Mcp-Name` must say what its body says, a header on two lines refused, or it is 400 with `-32020` ([ADR 039](../adr/039-mcp-serves-both-eras-on-one-endpoint.md)). |
| A browser page calling `/mcp` by DNS rebinding | Since 2.6.1 `/mcp` checks `Origin` in both eras before reading the request: none, a loopback one, or one in `AGNOSAI_MCP_ALLOWED_ORIGINS` or `AGNOSAI_PUBLIC_URL`'s own is let in, and any other, `null` and one on two lines among them, is 403. `Host` is never what is compared, as a rebinding page sends its own name in both ([ADR 039](../adr/039-mcp-serves-both-eras-on-one-endpoint.md)'s 2.6.1 addendum). ⚠ `*` in the list turns the check off |
| Slow-loris / connection exhaustion | 100 pool workers (`AGNOSAI_SERVE_WORKERS`) as the connection cap, and sandhi's 60 s accept-to-request budget (`SANDHI_SERVER_DEFAULT_REQUEST_MS`, sandhi 1.10.7), after which the request is answered 408. Rate limiting is on by default (below). A stream holds a worker for as long as it streams (ADR 014), and since 2.6.1 ends when its client hangs up or a write to it fails, within one 50 ms poll, so a client that leaves frees its worker; through 2.6.0 the crew stream held it for the crew's life whoever was reading ([ADR 041](../adr/041-a-stream-ends-when-its-client-or-the-server-does.md)) |

### 2. Tool Execution (WASM, cx, Python, Process, OCI)

Tools run untrusted code from community authors or legacy Python scripts.

| Threat | Mitigation |
|--------|------------|
| Memory exhaustion | WASM: kavach passes `-W max-memory-size` (64 MiB default) to wasmtime. OCI: `--memory` and `--pids-limit 256`. ⚠ **Process / Python: no memory limit.** There is no `setrlimit` here or in the Rust line (`src/sandbox/process.cyr`'s header), and kavach applies a cgroup only with a path agnosai does not give it |
| CPU exhaustion | WASM: 1B fuel (`-W fuel`) plus the sandbox deadline; every tier has a SIGKILL deadline |
| Spending exhaustion | Since 2.4.1 ([ADR 033](../adr/033-budgets-caps-and-crew-concurrency-are-enforced.md)): every model call is checked against its task's, its crew's and its tenant's budget, and a crew runs at most `max_concurrent_tasks` at once, 10 by default. ⚠ The server's default budget sets no token or cost limit, so a server takes what a request asks for unless its operator sets one or the request names its own. A budget is checked before each call, so calls in flight can pass it by one call each |
| A worker doing what its manager may not | Since 2.4.3 ([ADR 035](../adr/035-a-manager-assigns-its-workers-and-writes-the-crews-answer.md)) a hierarchical crew's worker holds only the tools its manager holds too, on a copy of its definition: its prompt lists those, the tool loop offers those, and it is ranked on those. The rest are withheld, named on its result and warned of, and nothing in the run widens a grant: neither the manager nor a model, only the crew's author, by giving the manager the tool. The manager itself is offered no tool when it writes the crew's answer, and every worker's answer reaches it as data (ADR 034) |
| Filesystem access | WASM: no filesystem capability; cx: landlock read-only on the interpreter's directory; Process / Python: landlock since 2.3.4 ([ADR 031](../adr/031-confine-the-process-and-python-tiers-in-agnosais-spawner.md)), read and execute on `/usr`, `/bin`, `/sbin`, `/lib*`, `/etc` and `/proc/self`, read and write on `/tmp`, `/dev/null`, `/dev/shm` and the call's work directory, nothing else. ⚠ A kernel with no landlock gets no ruleset, and agnosai warns at the first build. OCI: the container's read-only root and a tmpfs `/tmp` |
| Network access | WASM: no network capability; cx: a new network namespace, and the cx tier **fails closed** where one cannot be made (CHANGELOG 2.2.1); OCI: `--network=none` unless the policy allows it; Process / Python: **denied by default since 2.3.8** (ADR 031's 2.3.8 addendum), with no namespace: landlock refuses TCP connect and bind and kavach's network filter UDP, raw and the other families, all EACCES. A config allows it with `agnosai_process_config_set_network(c, 1)` or `agnosai_python_sandbox_with_network(s, 1)`. ⚠ **A kernel before landlock ABI 4 (Linux 6.7) leaves the tool the server's network** whatever its config says, and agnosai warns once; a namespace would hold it there and needs a user namespace, which Ubuntu 24.04+, docker and GitHub's runners refuse. Unix sockets stay on every kernel |
| Privilege escalation | Process / Python: NO_NEW_PRIVS, so no setuid or file-capability binary raises a tool's privileges. OCI: `--cap-drop=ALL` and `--security-opt=no-new-privileges`. Since 2.3.6 a process or Python tool holds no capabilities and has an empty bounding set, even under a root server (kavach 3.14.8), and a root server without `CAP_SETPCAP` refuses to run one. The image runs as `agnosai`, uid 10001 |
| Escaping through the kernel | Process / Python: kavach's exec seccomp filter, loaded last: `mount`, `umount2`, `pivot_root`, `unshare`, `setns`, `ptrace`, `process_vm_*`, module loading, `kexec_load`, `reboot`, `swapon`, `bpf`, `perf_event_open`, `userfaultfd`, `syslog` and the new mount API are denied, and so is a `clone` asking for namespaces. Every inherited descriptor is closed. OCI: docker's default seccomp profile |
| A model steered to a host its task did not declare (the system under test writes tool output, so a page can ask for it) | A consumer's gate ([ADR 025](../adr/025-a-tool-call-passes-a-consumers-gate.md)), asked at `agnosai_tool_execute_for` and of each URL, so no tool implements it and no path can skip it. ⚠ With no gate, a tool's target is still the model's to choose, under the SSRF guard alone |
| A model that reads what the system under test wrote, and does what it says | [ADR 029](../adr/029-the-tool-loop.md): nothing a tool writes reaches anything but the model. An agent's tools are fixed before its first call, and the limits are the process's, so a page cannot grant a tool or a budget. A result goes to the model inside the markers it is told never to obey, with `user_input` and `system_instructions` rewritten inside so a page cannot end them, and with control bytes and anything that is not UTF-8 taken out, so a binary page cannot cost a task its transcript. The injection table is a signal, and a flagged result is annotated, not held back: it is heuristic, and holding back on it would let a page blind its own QA. `agnosai_tool_gate_with_result` holds one back for a consumer who wants that. ⚠ A model can still be misled by what it reads: its calls are bounded, its claims are not, which is why a task's record keeps what the tool really said |
| A page one task read, carried into the next | Since 2.4.2 a dependent task is handed its dependencies' answers ([ADR 034](../adr/034-a-dependent-task-is-handed-its-dependencies-answers.md)), and a task that read a page may repeat it. So every answer is handled as a tool result is, whatever its task did: inside the markers, with `user_input` and `system_instructions` rewritten inside it and inside the dependency's id, a finding of the injection scan said outside them, and each answer cut at 32 KiB. Only the answer is handed over, never the calls and results of the tool loop that made it, and a failure's text never. ⚠ As with a tool result, a model can still be misled by an answer it reads |
| Calls that are harmless alone and leak together | `agnosai_tool_gate_with_batch` sees a turn's calls as a set, before any of them runs, and refuses all or none (ADR 029). ⚠ With no gate, nothing does |
| A model that never stops | At most 16 turns, 8 calls a turn, 32 KiB a result and 256 KiB of transcript a task, plus the crew's deadline and a cancel, each read between turns. A result's cost follows what is read of it, not its size. ⚠ A tool already running is not interrupted: the deadline is cooperative, and each tier's own deadline is what stops one |
| A redirect off the checked target | Every hop goes back through `agnosai_tool_url_ok` before its request is made (`agnosai_guarded_fetch_hops`), so the check does not end at the first 302. `load_testing` follows no redirect |
| A gate's admission reaching cloud credentials | An ADMIT gets a declared host past `agnosai_is_safe_url` and no further: 169.254/16, `fd00:ec2::254`, `metadata.google.internal`, `metadata.goog` and non-HTTP schemes are refused whatever the gate says (`agnosai_url_is_admissible`), and any verdict but PASS or ADMIT refuses. ⚠ Neither check resolves names, so a declared name that resolves somewhere it should not is the gate's to answer for |
| A private address spelled so the guard and the dialer read it differently | The guard judges a host by `sandhi_net_parse_ipv4`, the parser that dials, as well as the WHATWG reading, and a host must pass under both. Until 2.2.4 `0127.0.0.1` read as public octal and connected to loopback (CHANGELOG 2.2.4 *Security*). A name is compared with its root dots dropped, as a resolver reads it, so `foo.internal.` is `foo.internal` |
| Library injection (`LD_PRELOAD`) | All subprocesses strip `LD_PRELOAD`, `LD_LIBRARY_PATH`, `DYLD_INSERT_LIBRARIES`, `DYLD_LIBRARY_PATH` (`agnosai_sanitized_envp`) |
| Zombie processes | Every spawned child is waited on; the sandbox tiers enforce a SIGKILL deadline |
| Output flooding | 1 MiB stdout cap on the kavach paths; the spawner's capture is bounded by its deadline |
| A guest that hangs its worker (found 2.2.2) | Closed by kavach 3.14.4, pinned at 2.2.5: stdin goes out while stdout drains, and the deadline is read on every pass. Re-run here with real guests (`tests/sandbox_wasm.tcyr`): a guest that writes while it reads finishes 200,000 B in about 100 ms, and one that reads nothing is killed at its deadline with stdin still full. The 64 KiB refusal that stood in for this is gone |
| A guest that declares more than it should | Held to the Rust line's store limits, 10 instances, tables and memories, through `-W`, and refused at instantiation past them, so it never runs. A failed `memory.grow` traps. `tests/sandbox_wasm.tcyr` runs 10 and 11 of each it can reach. A core module is one instance, so `max-instances` is read from the config and not run |

### 3. LLM Provider Communication

HTTP requests to hoosh (the gateway), and through it to providers.

| Threat | Mitigation |
|--------|------------|
| API key leakage | Keys in environment variables, never logged or serialized |
| The gateway's URL in a result | A transport error names its failure kind and nothing else, since that text becomes a task's `output`, which a consumer serves and stores. A log line says which gateway from sandhi's parsed scheme, host and port, and withholds a URL holding an `@` or an odd host (`agnosai_hoosh_log_url_a`), at start-up too |
| Provider impersonation | The stdlib's native TLS with chain and hostname verification, nameConstraints enforced since cyrius 6.7.18 (CYRIUS-2026-0042), and extendedKeyUsage checked at every depth (CYRIUS-2026-0026) |
| Response injection | LLM responses treated as untrusted data; never executed as code |
| A caller growing the gateway by naming models | hoosh's heap never frees, and a catalog GET keeps about 1.7 KB per entry there. A refused id that no list could change reads nothing, and any other reads at most once a minute (ADR 030). ⚠ What a chat request itself leaves on hoosh is hoosh's (its ADR 013) |
| A gateway naming the model | An id the catalog lists goes on a request as a JSON string, through the serializer, and reaches nothing else, so a hostile list can choose a model and do no more. Ids over 255 bytes are dropped, a price that does not fit is no price, and a body over 2 MiB is a failed read. ⚠ agnosai trusts its gateway on this: a list can point Premium at the dearest thing it serves, and the only controls are `AGNOSAI_MODEL_*` and an agent's own `llm_model` (ADR 030) |
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

### 8. Crew logs on disk (`AGNOSAI_STATE_DIR`, [ADR 036](../adr/036-a-crew-keeps-a-log-and-resumes-from-it.md))

Since 2.5.0 a crew can keep a log, and the server keeps one for every crew when `AGNOSAI_STATE_DIR` is set. It holds
the crew's spec, every answer, and every tool call's arguments, and a start resumes what it finds there.

| Threat | Mitigation |
|--------|------------|
| Another local user reads a crew's answers or tool arguments | Each log is created `0600`, and opened `O_CLOEXEC`, so a sandboxed tool's child does not inherit it. The directory is the operator's: agnosai creates it only when it is missing, with the process's umask |
| A crew id names a path outside the directory | Every store verb refuses an id with a separator, a `.` component or a leading `.`, before a backend sees it ([ADR 008](../adr/008-durable-state-crew-id-validation.md), 2.5.0 note), and `list` drops one a backend hands back |
| A planted or edited log makes a start run a crew | ⚠ **Not mitigated: the directory is trusted.** Whoever can write it can have the next start run any crew, with the server's tools and gateway. Keep it as private as the binary's own configuration |
| A resume repeats a side effect | A task whose earlier attempt made a call that may have taken effect waits for a reviewer before it runs again; with no gate, or without a decision in five minutes, it fails and the call does not repeat. A call's intent is on disk, synced, before the call runs, and a call whose intent will not go down does not run |
| A torn or corrupt line | A line that does not parse is passed over, and a log with no spec it can rebuild is not run: a WARN at start |
| Two processes on one directory, the second resuming the first's live crews | The server claims its directory with an exclusive, non-blocking `flock` on `<dir>/.owner`, held for its life and dropped by the kernel however it exits, and a second refuses to start. A library caller that keeps logs (`agnosai_orchestrator_with_state_store`) claims its own store |

## Trust Boundaries

```
Untrusted                    Trusted
─────────────────────────────────────────
HTTP clients          →  API validation layer  →  Orchestrator
traceparent header    →  strict W3C v00 parse  →  span parent + sampling (honoured as-is)
WASM tool modules     →  kavach + wasmtime     →  Tool registry
cx tool bytecode      →  kavach (ns, landlock) →  Tool registry
Python / process tools →  agnosai's spawner + kavach's confinement (landlock, seccomp, NO_NEW_PRIVS) →  Tool registry
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

- **A crew's author grants its tools** ([ADR 029](../adr/029-the-tool-loop.md)): anyone who may `POST /api/v1/crews` can
  name a tool for a model to run. They could already call it on `/mcp`, under the same auth, so this widens what a
  model can do and not what a caller can. A consumer's gate is the place to narrow it.
- **Any caller who may `POST /api/v1/approvals` decides any pending task** (ADR 032): a decision names a task id, and
  the gate checks nothing about who sends it beyond the route's auth, as in the Rust line. Task ids are random UUIDs,
  listed only to callers of `GET /api/v1/approvals` under the same auth. A consumer that needs a reviewer per tenant
  or per crew decides through its own gate (`agnosai_orchestrator_with_approval_gate`).
- **A reviewed answer goes to the crew's watchers before it is approved** (ADR 032): `approval_requested` carries it,
  so a reviewer can judge it. A watcher already reads every answer through `token`, under the same auth, so this
  shows the stream nothing it did not already carry.
- **`delegate` hands out at most what its caller holds** since 2.4.3 ([ADR 035](../adr/035-a-manager-assigns-its-workers-and-writes-the-crews-answer.md)).
  Through 2.4.2 a delegated crew's agents held what their own definitions named, however little the delegating agent
  held. Now the agent delegated to holds only the tools its caller holds too, and its crew at most what the caller's
  task has left, and a task with nothing left is refused. ⚠ **The budget is a ceiling taken when it delegates**, not
  a share: the delegated crew outlives the call, so what it spends is not recorded against its caller, and a task that
  delegates and spends on can together pass what it had. Nothing bounds how deep it nests, though each nested crew
  holds no more than the one above it had left. It is opt-in and off in the binary. Since 2.4.1 the crews it starts
  wait their turn with every other (ADR 033), so a model nesting them cannot run more than `max_concurrent_tasks` at
  once.
- **Each WASM call keeps a few KiB** on the global heap, 3,160 B in `tests/sandbox_wasm.tcyr`, so a caller that can
  drive WASM tools grows this process by that much per call until a restart. Through 2.3.2 it was about 1 MiB a call,
  kavach's capture buffer, which kavach 3.14.7 gives back and agnosai 2.3.3 requires. Since 2.3.4 the WASM tool's
  payload and output are in the call's arena, and what is left, about 2.9 KB, is kavach's run, inside kavach.
  A process or Python tool call in the call's arena keeps 0 B (`tests/sandbox_confine.tcyr`).
- **A crew arena is reused** ([ADR 028](../adr/028-a-crew-lives-in-an-arena-its-consumer-gives-back.md)):
  the memory a crew lived in serves the next one, which in a multi-tenant consumer may be another
  tenant's. It is zeroed on release, every byte handed out, so no allocation can read a previous
  crew's prompts or keys, and `tests/orch_crew_arena.tcyr` asserts that. What it cannot stop is a
  stale pointer: a consumer that reads a released crew reads zeros, or the next crew's bytes once the
  arena is reused. Clearing its own pointers before `agnosai_orchestrator_release_crew` is the
  consumer's half of that contract, and a crew that does not opt in cannot meet this at all.
- **TOCTOU on tool paths**: since 2.3.4 the process and Python tiers pin a tool's binary by
  descriptor before the fork and exec through it (`kv_exec_pin`, ADR 031), so what runs is the
  file that was checked. ⚠ The OCI tier's runtime (`docker`, `podman`) is still resolved against
  `PATH` and run by path, unconfined, as the daemon builds the container. Mitigated by a trusted
  `$PATH`.
- **What confinement leaves a process or Python tool** (ADR 031): it reads `/etc` and the system's
  trees, which hold no secret a non-root uid can read but do describe the host. It keeps the
  server's unix sockets, and its network too where its config allows it or the kernel is before
  landlock ABI 4. It shares `/tmp` and `/dev/shm` with every other tool and with the server,
  which is where staged WASM modules sit when `TMPDIR` is unset; a module goes when its tool
  does (ADR 019's 2.3.5 addendum). A process it starts cannot read its own `/proc`, as the
  `/proc/self` rule names the confined process alone.
  A host where kavach cannot build its seccomp filter refuses every such tool. A kernel without
  landlock runs them with no filesystem rules, and agnosai warns once. An operator turns
  confinement off per config with `agnosai_process_config_set_confinement(c, 0)` or
  `agnosai_python_sandbox_with_confinement(s, 0)`, which is for a command they wrote.
- **Fleet coordinator election**: lowest-id election, and any non-stale term is adopted (surface 4).
  No Byzantine fault tolerance. Suitable for trusted internal networks.
- **Caller-controlled trace context** (surface 7): any caller of a traced route
  chooses the trace its spans join and can suppress their export with sampled
  flag `0`. Accepted because joining the caller's trace is the point of the
  feature and nothing but OTLP spans is lost; the operator switch that would
  restart an untrusted inbound context is a roadmap item.
