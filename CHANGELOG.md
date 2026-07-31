# Changelog

All notable changes to AgnosAI will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- **Toolchain pin 6.5.2 → 6.5.3.** `lib/` is byte-identical between the two tags
  (`git diff 6.5.2 6.5.3 -- lib/` is empty), so the bump moves no stdlib source and
  needed no re-verification beyond a full rebuild. It is bugfix-only upstream —
  correct diagnostic line numbers after an `include`, and an `install.sh` fix — and
  it clears the `manifest-pin: 6.5.2 (drift — wrapper is 6.5.3)` banner the
  installed CLI printed on every invocation. 43/43 suites still green after.

### Fixed
- **`cyrius.cyml` pinned `ai-hwaccel = "2.3.15"` while `lib/` and `cyrius.lock`
  carried 2.3.16.** The manifest was the only stale copy, so a clean checkout would
  have resolved **2.3.15 — the version that still carries the bayan-1.3.0
  `json_v_parse_str` break** this project filed and consumed the fix for. The build
  in the working tree was correct; it simply was not reproducible from the manifest.
  Now pinned 2.3.16, matching upstream's current tag.
- **docs/development/state.md — table drift.** The gates all reproduced exactly
  (2684 assertions across 43 files, 752/752 coverage, fmt/vet/deny clean), but every
  hand-transcribed table had drifted: six per-suite assertion rows (`id` 26→37,
  `llm_hoosh` 120→124, `orch_audit` 52→54, `orch_crew_runner` 184→188,
  `orch_orchestrator` 51→49, `server_output_filter` 67→63 — the deltas happened to
  cancel, which is why the headline stayed right), the locked-dep count (79→105),
  libro's version (2.8.2→2.8.4), and the duplicate-fn count (35→36).
- **`path_exists` duplicate-fn verdict was inverted.** state.md said kavach's wins
  and that the two are "same contract, different implementation". Both are wrong,
  and both were disproved by running a probe: **ai-hwaccel's wins** (it is included
  later), and the contracts genuinely differ — kavach's `sys_access(path, 0)` tests
  existence only, while ai-hwaccel's delegates to `file_exists`, which opens
  `O_RDONLY`. A path that exists but is unreadable answers 1 under one and 0 under
  the other. agnosai has no `path_exists` call sites, so nothing misbehaves today.
- **`_agnosai_is_digit` is defined twice in our own source** —
  `src/server_ssrf.cyr:39` and `src/server_output_filter.cyr:140` — and is the 36th
  duplicate-fn warning, the only one that is not a dep's. The bodies are
  semantically identical today, so nothing misbehaves, but two copies of a scanner
  that agree by accident is the condition ADR 007 exists to prevent. Documented for
  hoisting at the next touch of either module.
- **docs/development/roadmap.md** still described M5 as "16 of 18 bites" with
  `durable_state (→ patra)`, a mapping the port plan corrected on 2026-07-29 and
  which the roadmap never picked up. M5 is complete and touches no patra.
- **CLAUDE.md work-loop step 2 said bare `cyrius lint`**, which takes a file — bare
  it prints usage and exits 1, so a gate written that way lints nothing. Step 11
  gated on a "recipe" file that exists nowhere in the repo; it now names the real
  sync set, and flags that `scripts/version-bump.sh` is the un-ported Rust-era
  script (it still edits a root `Cargo.toml` that no longer exists).
- **docs/development/cyrius-port-plan.md** was still headed "Phase 0 in progress"
  and listed the `vec_sort_by` / `vec_select_nth` ask as "still to file" — it had
  been filed the same day it was written (`2026-07-28-agnosai-no-nlogn-sort-in-stdlib.md`).

### Added
- **server_auth (M6 bite 3) — the shared-secret half of `auth.rs`.** `AuthConfig`,
  `JwtConfig` and its builders, case-sensitive `Bearer ` extraction, the
  `HeaderValue::to_str()` visible-ASCII gate, and the shared-secret comparison.
  All five of the oracle's shared-secret tests port directly, plus 47 assertions
  the oracle has no equivalent for — 52 in total.

  **Shaped as `fn(config, inputs) -> status`, not as a transport handler.** The
  oracle's five tests are `#[tokio::test]` + axum `oneshot` only because its
  middleware signature is async; the decision it makes is synchronous. Writing it
  as a pure function makes the whole thing testable before any sandhi adapter
  exists, and it is the pattern the remaining `routes/*` bites should follow —
  most of M6's 43 remaining `#[tokio::test]`s are async for the same incidental
  reason.

  **The secret comparison is constant time; the oracle's is not.** `constant_time_eq`
  (`auth.rs:18-31`) bounds its loop with `a.len().max(b.len())` while its own doc
  comment claims "no early return on length mismatch that would leak secret
  length" — the content compare is constant time, the loop bound is not, so an
  attacker who controls the token length can recover the secret's by timing.
  `ct_eq_bytes_lens` is not the fix either; it early-returns on a length mismatch
  (`lib/ct.cyr:76`), a sharper signal of the same fact. The port compares SHA-256
  digests over a fixed 32 bytes instead. **This is not a wire divergence** — the
  accept/reject set is byte-identical, since `sha256(a) == sha256(b)` iff `a == b`
  — only the timing leak is gone. Recorded as
  [ADR 009](docs/adr/009-auth-constant-time-secret-compare.md).

  **The JWT branch is a loud 500, never a silent pass.** `validate_jwt` is bite 4.
  A stub returning 401 would be indistinguishable from a working rejection and one
  returning 200 would be an authentication bypass, so the unported path answers
  `HTTP_INTERNAL` and `_t_jwt_path_is_a_loud_stub` pins that it is never `HTTP_OK`.
  Read state.md's "Four decisions waiting on the maintainer" before writing bite 4:
  `iss`/`aud`-absent-passes, the `exp: u64::MAX` fixture, the array-`aud` 401, and
  jsonwebtoken's 60-second default leeway are all deliberate calls.

  Three oracle behaviours reproduced rather than fixed, each pinned by a test: the
  `Bearer ` prefix is **case-sensitive** (RFC 7235 says the scheme is not, but the
  oracle's `starts_with` is, so `bearer ` is a 401 in Rust today); `AuthConfig`'s
  default is **fail-open**; and a header failing `to_str()` takes the
  missing-header arm rather than being compared.

- **M6 (`server`) — first bite.**
  - **server_prometheus** — six counters plus the Prometheus text exposition `/metrics` serves.
    **Atomics, not a mutex.** The oracle's counters are `AtomicU64`/`Relaxed`, and the port uses
    `atomic_fetch_add` for the same reason rather than locking: the crew runner's parallel and DAG
    modes record from real worker threads, and an uncontended mutex pair costs ~394 ns here against
    a measured **5 ns** for the atomic — ~79× on a path that fires once per task. The one
    non-additive operation, the active-crews decrement, is a CAS loop, since Cyrius has no
    `fetch_update`; it saturates at zero, because a plain decrement would *wrap* and the gauge would
    read as nonsense rather than merely wrong.
    **Cost is integer micro-USD end to end.** The oracle already stores micro-USD, but its entry
    point takes an f64 USD and multiplies while `gather` divides back to format `{:.6}` — two float
    conversions bracketing an integer store. The port takes micro directly (the one signature
    change) and formats by splitting the integer, so the value that arrives is the value stored and
    printed. That matters because hoosh 2.6.0 reports `usage.cost_micro_usd` as an integer and the
    port carries micro-USD everywhere: the f64 round trip would have been the *only* place in the
    cost path where representation error could enter. Tests pin `0.000001`, `0.999999`, `2.000000`
    and `1234.567890` rendering exactly.
  - **server_output_filter** — the return leg of `server_prompt_guard`: that guards what goes *to*
    the model, this scans what comes *back*. Detection and redaction of system-prompt leakage, ten
    API-key prefixes, and PII (email, phone, SSN). Substring-based rather than regex, which the
    port has no choice about since cyrius 6.5.0 removed the `regex_*` surface — and which is what
    the oracle chose anyway.
    **Three oracle behaviours are reproduced rather than fixed**, each pinned by a test so a
    tidy-up fails loudly: a system prompt of **50 bytes or fewer is never checked** (the window
    loop's range is empty at that length, and the oracle's own test asserts this as intended); the
    **last window is never checked**, so leaking exactly the prompt's tail goes unseen; and
    **`Bearer ` redacts only itself**, because its own trailing space is the first whitespace the
    span-bounding scan finds — the token survives and is only removed if it happens to match
    another prefix like `sk-`.
    **One divergence, and it is a fix.** The oracle's SSN redactor walks bytes and pushes each as a
    `char`, silently mangling any multi-byte UTF-8 that passes through — its email redactor does
    not, because it walks `chars`. Cyrius Strs are byte slices with no re-encoding step, so
    non-ASCII survives; a test drives a two-byte character through redaction to prove it.

- **tools** (M4, Phase 3, in progress) — `src/tools.cyr` hub plus two submodules:
  - **tools_native** — `agnosai_tool_*`: ParameterSchema, ToolSchema, ToolInput, ToolOutput, and
    the tool itself. The oracle's `NativeTool` **trait** becomes a function-pointer vtable
    (schema/execute plus an opaque ctx, dispatched with `callptr`) since Cyrius has no traits,
    and `execute` becomes **synchronous** — there are no futures, and under
    `sandhi_server_run_pooled` a blocking tool body on a worker thread is the direct equivalent
    of an awaited future on a tokio task.
  - **tools_registry** — `agnosai_tool_registry_*`: registration, lookup, allow-list gating.
    The oracle's lock-free `DashMap` becomes a hashmap behind a **futex mutex**, which is
    mandatory rather than optional: `run_pooled` makes every worker its own OS thread, so an
    unguarded write during a concurrent read would corrupt the table. Schema callbacks run
    outside the lock, since a tool's `schema_fp` is arbitrary user code.
- **tools_builtin_basic** — the `echo` and `json_transform` builtins, registering through the
  registry and gated by the allow-list. `echo` returns the whole JSON value rather than a string,
  matching the oracle's `Value` clone.
- **tools_builtin_load_testing** — the `load_testing` builtin: HTTP load generation against a
  target URL with concurrent users, reporting throughput, error rate, status-code histogram and
  min/avg/p50/p95/p99 latency. **The first production user of the port plan's blocker #3 arena
  pattern.** One OS thread per simulated user — a load generator that ran sequentially would not
  be one — with each worker owning two arenas: a persistent one, sized from its request budget,
  holding its latency samples, and a scratch one `reset_via`'d after every request. The scratch
  arena is load-bearing, not a refinement: a single arena would accumulate every response body
  for the whole run, which is unbounded growth the oracle does not have, since Rust drops each
  response as it goes.

  Three deliberate divergences, all documented in-module:
  - The percentile index is the oracle's `(len * p / 100).min(len - 1)`, **not**
    `order.cyr`'s nearest-rank convention. For n=100 they differ — index 50 against 49 — and the
    reported figure has to be the oracle's.
  - Throughput and error rate are carried as integers (thousandths of a request/second, parts per
    million) and converted to float only at the wire boundary, the same treatment money gets.
  - Status counts live in a vec of `[code, count]` pairs rather than a map, because the stdlib has
    no `map_u64_keys`. Benchmarked rather than assumed: see **Performance**.

  The worker loop checks `vec_push_a`'s return value. The arena sizing has ~2.5x headroom, so
  exhaustion is unreachable through the public constructor — but `vec_push_a` returns -1 *and does
  not push* when an arena is full, and the loop exits on `vec_len(latencies) >= budget`. Ignoring
  the failure would mean spinning until the deadline issuing real HTTP requests whose results are
  all discarded: maximum load generated, nothing measured.

  The SSRF guard runs before anything touches the network, which means the tool cannot be aimed at
  loopback. That is deliberate, and it is why the oracle's two axum-mock-server tests do not port
  directly — the suite drives the real thread fan-out against a synthetic executor instead, and
  `scripts/stack.sh check` covers the network seam separately.
- **tools_builtin_security_audit** — the `security_audit` builtin: HTTP security-header analysis,
  a CORS probe via OPTIONS, information-disclosure detection, scoring and a risk band. Split at
  the network boundary — `agnosai_audit_analyze` takes two already-fetched header sets and
  `agnosai_run_security_audit` is the thin shell that fetches them — which is what makes the
  oracle's five loopback-mock-server tests portable, since the tool's own SSRF guard rightly
  refuses loopback. Scores are carried as integer percentages and converted to f64 only at the
  wire boundary, the same treatment money and load_testing's throughput get; the oracle's
  arithmetic is integral at every step, so an f64 carrier could only drift.

  **Redirect handling deliberately diverges from the oracle in both directions —
  [ADR 007](docs/adr/007-audit-redirect-revalidation.md).** reqwest follows up to 10 redirects
  and validates only the URL the caller supplied, so a target that passes `is_safe_url` and then
  answers `302 Location: http://169.254.169.254/` walks the oracle into the cloud metadata
  service and reports its headers back — an SSRF bypass in a tool whose job is finding them.
  sandhi's opposite default (never follow) would have been differently wrong: auditing
  `http://example.com` when it redirects to HTTPS would score the redirect stub and report a
  well-configured site as 0/7, critical. The port follows hops and re-runs the guard on each one,
  refuses an https→http downgrade, fails closed on a `Location` it cannot resolve confidently,
  and reports a refused hop as a distinct error rather than a generic failure.

  Three inherited defaults were corrected rather than absorbed, each of which would have produced
  a silently wrong answer:
  - `max_response_bytes` is raised off sandhi's 256 KiB, which treats an over-cap response as a
    hard protocol error. The oracle cannot fail that way at all — reqwest's `send()` resolves on
    the response head and never reads the body — so any homepage over 256 KiB would have returned
    "security audit failed" where the oracle returns a full result.
  - The 15s budget spans the whole redirect chain, matching `Client::timeout`, rather than being
    re-armed per hop. Per-hop would have allowed 165 seconds against the oracle's 15, twice over.
  - Scheme comparison on the security paths is case-insensitive, because `sandhi_url_parse` is and
    therefore `is_safe_url` accepts `HTTPS://`. A byte-exact test would have skipped the downgrade
    refusal and sliced the origin one byte short, resolving a relative `Location` against
    `HTTPS:/` and pointing the next request at a different host.

  Information disclosure honours the oracle's `let Ok(v) = val.to_str()` gate: a header value
  carrying a non-visible-ASCII byte is skipped, so it raises no vulnerability and costs no points.
  Without the gate such a target would score 5 below the oracle.
- **tools_agnos** — the shared client behind the nine AGNOS ecosystem tools. synapse, mneme and
  delta are nine tools with one shape (a cloned `reqwest::Client`, a `base_url`, a JSON GET or
  POST, and two fixed error strings); the Rust side factored out only the `OnceLock<Client>`
  because everything else was cheap to repeat behind a derive. Repeating it nine times in Cyrius
  is not cheap, and this is the third instance, which is where CLAUDE.md says the abstraction is
  earned. Carries the `application/x-www-form-urlencoded` serialiser reqwest's `.query()` gave
  the oracle for free — the stdlib has no percent-encoder — and the
  `contains('/') || contains("..")` path-segment guard mneme and delta both apply.

  **The transport is a function pointer**, so the tests drive all nine tools end to end —
  parameter extraction, URL construction, query encoding, body construction, the traversal
  guards, the response reshaping — with no service running. The oracle's own suites test only
  names, descriptions and schemas, because every execute path there needs a live loopback service.

  **These tools deliberately do NOT run the SSRF guard.** They target AGNOS services on loopback
  by design, so `agnosai_is_safe_url` would reject all three default base URLs; the test asserts
  exactly that, so the omission reads as a decision rather than an oversight. The guard belongs
  on tools that fetch a URL the *caller* chose. What the caller does control — the path segments
  — is guarded where the oracle guards it.

  Two inherited defaults corrected: a 30s timeout, where reqwest's `Client::new()` applies none
  at all and a hung service would hang the agent forever; and `max_response_bytes` raised off
  sandhi's 256 KiB for the same reason as in security_audit.
- **tools_builtin_synapse** — `synapse_infer`, `synapse_list_models`, `synapse_status` against the
  OpenAI-compatible controller on :8420. `synapse_infer`'s completion extraction reproduces the
  oracle's `.unwrap_or("")` tolerance exactly: a missing `choices`, an empty array, a missing
  `message`, a non-string `content`, or a response that is not an object at all all yield an empty
  completion rather than an error, because the raw response ships alongside it.
- **tools_builtin_mneme** — `mneme_search`, `mneme_get_note`, `mneme_create_note` against the note
  store on :8400. `tags` is forwarded whatever its JSON type, matching the oracle's
  `parameters.get("tags").cloned()`, which never type-checks; validating would reject a request
  the oracle accepts.
- **tools_builtin_delta** — `delta_list_repos`, `delta_trigger_pipeline`, `delta_get_pipeline`
  against the code platform on :8070. `delta_trigger_pipeline` sends `{}` when no branch is given:
  the parameter's own description says "defaults to main", but the oracle inserts nothing and the
  code is what ships. Guard order is the oracle's array order, so with both `owner` and `repo`
  invalid it is `owner` that is named.
- **orch** (M5, Phase 4, in progress) — `src/orch.cyr` hub. Two modules so far:
  - **orch_output_validation** — the structured-output check a task's `output_schema` drives, plus
    the retry prompt built from a failure. `ValidationResult` flattens to the port's
    `Option<String>` convention, so **0 means Valid**. The fence extractor reproduces two
    behaviours that are one character apart in Rust: the `?` on the closing-fence lookup abandons
    the whole search rather than trying the next opening marker, while an *empty* block does fall
    through — and the fall-through's re-scan can capture a block that still contains a fence,
    which is pinned as-is rather than tidied. Divergences are message text only: the parse error
    carries bayan's detail behind the oracle's wrapper wording, and the schema renders in
    insertion order where serde_json (BTreeMap-backed, no `preserve_order`) sorts keys.
  - **orch_pubsub** — topic pub/sub with `*` (one segment) and `#` (zero or more) wildcards.
    A pattern maps to a **vec of per-subscriber channels** rather than the oracle's single
    broadcast sender, because Cyrius channels are single-consumer: a value one receiver takes is
    gone for the rest. The observable contract is unchanged — every subscriber sees every matching
    message, and `pattern_count` still counts patterns, which is what the 10,000 cap is expressed
    in. The oracle's 16-entry stack-array fast path is not reproduced; it is an allocation
    optimisation with no observable difference.
  - **orch_multi_tenant** — per-tenant token/cost/concurrency limits and the check that enforces
    them. `max_cost_usd` becomes integer micro-USD, which lands more cleanly here than anywhere
    else it has been applied: `TenantBudget` derives no `Serialize`, so there is no wire boundary
    to convert at and the field exists purely to be compared. Every limit is breached by
    *exceeding* it, never by reaching it — all the oracle's comparisons are `>`, and the check
    order (unknown tenant, tokens, cost, concurrency) is observable when several are breached at
    once.
  - **orch_ipc** — Unix-socket IPC, 4-byte big-endian length prefix then JSON. Connection setup
    delegates to majra's `ipc_bind`/`ipc_accept`/`ipc_connect`, which own the `sockaddr_un`
    construction and the agnos fail-closed path. **The framing does not**, for two reasons that
    would both have been silent: majra caps a frame at 1 MiB where the oracle allows 16 MiB — and
    the oracle has a test for a >64 KiB payload precisely because large frames are expected — and
    majra collapses every failure into one error where the oracle distinguishes six, two of which
    a caller acts on differently (a clean peer disconnect is not a fault; an over-cap frame is a
    misbehaving peer). The port also keeps the oracle's zero-length-frame rejection, which majra
    lacks; without it a peer can hold a reader in a loop that consumes four bytes and yields
    nothing.
  - **orch_scoring** — five weighted factors scoring an agent's fit for a task. The weights are
    the **constants**, not the rustdoc: `score_agent`'s doc claims 0.40/0.30/0.15/0.15 over four
    factors while the `WEIGHT_*` constants are 0.35/0.25/0.10/0.15/0.15 over five, and the
    oracle's own `expected_score` test helper recomputes from the constants. **Personality always
    scores the neutral 0.5** — that is the oracle's own `personality: None` arm, which is the only
    value the default Rust build ever produced and what its test helper hardcodes; the
    bhava-backed trait arms defer with bhava, and `agnosai_personality_score` is the single
    function to fill in when it lands. A perfect match therefore scores 0.925, not 1.0.
    `rank_agents` breaks ties on ascending index because the oracle sorts with `sort_by`, a
    **stable** merge sort, while `order.cyr`'s heapsort is not stable — without the tie-break,
    which of two equally-good agents gets the task would vary with the sort's internal swaps.
  - **orch_scheduler** — five FIFO priority tiers plus DAG-aware ordering, Kahn's algorithm for
    both. Determinism runs in two directions here and both are reproduced deliberately.
    `kahn_sort`'s two sorts — the zero-in-degree seed and each node's successors — exist so a
    HashMap's arbitrary iteration order cannot leak into the output, including the detail that
    later waves are *appended* without re-sorting, so the result is sorted within each wave rather
    than globally. `ready_tasks` and `topological_sort_tasks` are the opposite: the oracle leaves
    their tie order genuinely unspecified (a HashMap collect, and a `BinaryHeap` breaking ties on
    raw UUID bytes), so the port picks a stable documented order instead — reproducible run to
    run, which the oracle is not, and consistent with its contract either way. The adjacency
    values are sets, not lists: a duplicated edge must not double-count an in-degree, or the
    target never becomes ready.
### Changed
- **toolchain: cyrius `6.4.86` → `6.5.2`** (`cyrius.cyml`), folding in bayan 1.3.0, sandhi 1.9.7
  and sakshi 2.4.7.
  - **bayan 1.3.0 renames the cstr+len parsers `_str` → `_buf`**, so **23 call sites moved from
    `bayan_json_v_parse_str` to `bayan_json_v_parse_buf`** across nine `src/` modules and seven
    suites. The bodies are byte-identical — a pure rename, no semantic change. The rename exists
    because `X_str` is a *reserved overload slot*: Cyrius routes `X(a, …)` to `X_str` whenever `a`
    is Str-typed at the call site, so a cstr+len form may never occupy that name. While it did,
    every bare `bayan_json_v_parse(someStr)` in the ecosystem was silently rewritten into a 1-arg
    call to a 2-arg function and returned 0 for valid JSON. Here it was a compile error rather
    than a silent break, so no call site could be missed.
  - **sandhi 1.9.7 closes port-plan blocker #3, and it has now reached the port.**
    `lib/sandhi.cyr` carries `sandhi_server_options_req_arena` / `_get_req_arena` (a per-worker,
    per-request arena, default off), `sandhi_server_request_arena`, and the allocator-threaded
    `sandhi_router_dispatch_a` / `sandhi_server_router_handler_a` — all five verified present.
    The routing path can now allocate in a rewindable arena instead of the no-free global bump,
    so **M6 can be built against it rather than around it.** Residual, filed upstream and not
    sandhi's to fix: 16 B/response from the `Result` that `lib/net.cyr`'s `sock_send` returns.
  - **sakshi 2.4.7** in the bundle, though `lib/sakshi.cyr` still lands at 2.4.3 and the shadow
    warning persists — each git dep vendors its own sakshi distribution and `cyrius deps` copies
    them last-write-wins (sigil and kavach carry 2.4.3, tyche 2.2.10). Not a correctness problem:
    the diff is three added public verbs plus `_`-prefixed internal churn, and the older bundle is
    a superset of what anything here calls.
  - Two upstream issues filed the same day were **re-verified as still present on 6.5.2** rather
    than assumed fixed: `mutex_unlock`'s unconditional `FUTEX_WAKE` (392 ns, unchanged) and
    `fmt_int_buf`'s `i64::MIN` corruption (still emits `{"n":-}`). Every orchestration benchmark
    re-ran within run-to-run noise of its 6.5.1 value, which independently confirms the mutex
    path did not change.

- **Identifiers are canonical strings, not raw 16-byte UUID buffers.** `agnosai_task_id`,
  `agnosai_crew_id`, `agnosai_message_id`, task dependencies and assigned-agent ids now hold the
  36-char canonical form; `agnosai_uuid_v4_str` and `agnosai_uuid_canonical` are what constructors
  and parsers use. The byte-level verbs (`agnosai_uuid_v4`, `_eq`, `_to_str`, `_parse`) are
  unchanged for callers who genuinely want bytes.

  The buffer form was the natural port of `uuid::Uuid` and it was a standing trap: Cyrius has no
  types to stop one reaching `str_eq` or a `map_new_str` key, where its first eight bytes are read
  as a string length. That is not a compile error and not a clean crash — it is a segfault three
  call layers away, and it cost a full debugging session in `orch_scheduler`, whose `id_to_key`
  map looks like it should take an id directly because the oracle's does
  (`HashMap<TaskId, String>`). Storing the canonical string removes the trap instead of
  documenting it: ids compare with `str_eq`, key a map directly, and serialise with no conversion.

  **The wire form is unchanged** — it was already the canonical string, so every `to_json` /
  `from_json` round trip is byte-identical. Parsing still validates and now also normalises case,
  matching the `uuid` crate (parse any case, Display always lowercase).
  - **orch_budget** — token/cost enforcement checked before every inference call. **The one
    module in the port that does not use micro-USD**: the oracle carries its own unit, 1/10000
    USD, chosen so an atomic counter needs no float — and `BudgetExceeded::Cost` derives
    `Serialize` with `used_units`/`limit_units`, so the divisor is **on the wire** and cannot be
    quietly widened. The two units meet at the constructor, where a micro-USD limit converts with
    a ceil; since one cost unit is exactly 100 micro-USD that is integer ceil-division, making the
    port's conversion *more* exact than the oracle's f64 multiply-then-ceil. Limits bite at `>=`,
    so a tracker that has spent exactly its allowance refuses the next call — the opposite of
    `multi_tenant`'s `>`, and both are the oracle's. The atomics become plain fields; a concurrent
    `record_tokens` pair can lose an update where `fetch_add` would not, which is noted in-module
    rather than hidden and is harmless for a coarse ceiling.
  - **orch_approval** — human-in-the-loop gates for High-risk (and optionally Medium-risk) task
    results. **Every failure mode rejects**: a timeout, a cancelled gate, a gate at capacity, and
    an unrecognised decision spelling all come back `Rejected`, which is the oracle's behaviour
    across three separate arms and the only safe default for a gate whose job is to stop something
    unreviewed. `Rejected` is deliberately 0 so a zeroed field is a refusal, not an approval.

    `oneshot::Sender/Receiver` becomes a capacity-1 channel — exact, since a oneshot *is* a
    one-slot channel — and `tokio::time::timeout` becomes a polling wait with a 25 ms interval,
    because there is no runtime to race a sleep against a future. That change forced one ordering
    inversion worth naming: `submit_decision` sends **before** clearing the pending entry, the
    reverse of the oracle. The oracle's waiter awaits a future so the map entry is irrelevant to
    it; this waiter polls and uses the entry as its liveness signal, so removing first would let a
    poll landing between the two calls see an empty channel *and* no pending entry, conclude the
    gate was cancelled, and reject a decision that was about to arrive.
  - **orch_plan_cache** — an LRU+TTL cache of agent assignments, task ordering and model choices,
    keyed on a normalised hash of the crew spec. **The hash algorithm is not the contract; the
    normalisation is** — the oracle keys on `std::hash::DefaultHasher`, an explicitly unstable std
    detail, and `PlanKey` derives no `Serialize`, so the value never leaves the process. What is
    observable, and is reproduced, is that reordering a spec does not change its key.

    **One deliberate divergence closes a real collision the oracle has.** The oracle delimits each
    individual string but nothing delimits the agent section from the task section, so
    `(["a","b"], ["c"], m)` and `(["a"], ["b","c"], m)` feed the hasher identical bytes and
    collide — needing only an agent key that reads like a task description. That is not a
    hash-quality nit: two different crews share a cache entry, so one gets the *other's* agent
    assignments and task ordering. This port folds each section's element count in first. It is
    free, because nothing outside the process can observe the value. Found by an assertion written
    to check the property rather than to confirm the implementation.

    Re-inserting an existing key replaces rather than appends — the oracle gets that from
    `HashMap::insert`, and without it a hot key would fill the cache with copies of itself. An
    expired entry is removed on read, and eviction takes the *first* minimum `last_accessed`, so
    entries sharing a coarse-clock timestamp evict earliest-inserted first, matching `min_by_key`.
  - **orch_memory** — a per-agent conversation buffer for tasks needing more than one round trip,
    with three eviction strategies. Two asymmetries pinned because they read like bugs and are
    not: **`Full` ignores a configured cap entirely** (the cap is inert unless a trimming strategy
    is chosen), and **`HeadTail` at `max_messages == 1` keeps the LAST message, not the first** —
    sacrificing the head the strategy is named for, since there is no room for both ends. The trim
    guard is `<=`, so a buffer sitting exactly at its cap is untouched, and trimming runs on every
    push so the buffer is never observed over its cap.
  - **orch_hierarchical** — manager-driven delegation, one assignment per task in task order.
    **The manager is the fallback, never a candidate**: it is excluded from the worker pool so it
    cannot win a task on score, but with an empty pool it takes every task itself — so a crew with
    only a manager still runs rather than erroring. A test pins the sharp version of that: a
    perfectly-matched manager loses to a poorly-matched worker, which is the whole point of the
    mode. Each task is ranked independently, so one agent can take several and there is no
    round-robin or capacity limit.
  - **orch_orchestrator** — the crew registry and lifecycle, **completing M5's 15 modules**.
    Accepts a spec, audits `crew_accepted` **before** registering it, registers it Pending, runs it
    through a `CrewRunner`, audits `crew_finished`, stores the final state, then removes the cancel
    token and the crew's event channel. That order is the contract: getting it wrong still produces
    a working crew run and a broken audit trail, so the tests pin the sequence rather than the
    outcome.
    A **cyclic spec never reaches the registry** — it audits only the acceptance and returns no
    state, because the oracle's `?` propagates before anything can be recorded as finished.
    Cancelling an **unknown** crew is an error rather than a silent no-op.
    Eviction at `MAX_RETAINED_CREWS` takes only **finished** crews, so a registry full of running
    ones grows past the cap rather than dropping live state.
    The seam removes what was never agnosai's: the shared `ResponseCache` and `CostTracker` are
    gone, and the `OnceLock` lazy client init with them — the port's client is two Strs, so there
    is nothing expensive to defer.
  - **orch_crew_runner** (bites 2 and 3 of 3) — `execute_task` and the `CrewRunner`, completing
    **M5**. The runner carries the crew spec plus the optional client, event sender, cancel flag
    and audit chain, and dispatches to the three process modes.
    **Parallel and DAG waves use real OS threads** — `alloc` has been thread-safe since cyrius
    6.0.64, which is what makes that viable. There is no semaphore in the port, so the batch size
    *is* the concurrency limit: tasks run in batches of `max_concurrency`, every thread joined
    before the next batch, none left detached. A thread that fails to spawn runs inline rather than
    dropping the task, because a missing result would silently shorten the crew's output.
    Cancellation is a shared flag pointer, checked before each sequential task and again inside
    each worker — so a crew cancelled mid-wave does not start work that was merely queued.
    Behaviours pinned because they are easy to get wrong: an **empty crew completes** (`all()` over
    an empty set is true, so it must not read as failed); a **cyclic DAG returns no state at all**,
    only an error, so a consumer can tell "the spec was wrong" from "the work failed"; only
    **successful** tasks enter the DAG's completed set, which is what makes a failure cascade;
    **hierarchical falls back to sequential** with a warning, matching the oracle's unimplemented
    Phase 2 rather than wiring in `orch_hierarchical`'s delegation; and the **parallel path does
    not audit**, reproducing the oracle's asymmetry rather than "fixing" what a consumer's trail
    contains.
    `execute_task` carries the placeholder arm, the prompt-guard sanitisation of all four
    user-supplied fields, the retry seam via `callptr`, and the output-schema validation loop —
    which rebuilds each retry prompt from the **original**, never the accumulated one, so the
    prompt cannot grow exponentially. `metadata` reports **`cost_micro_usd`**, not the oracle's
    f64 `cost_usd`; the key is **omitted entirely** when the gateway reports no cost, because 0 is
    a real cost for a local model and a fabricated zero would be indistinguishable from a free
    inference.
  - **orch_audit** — a tamper-evident HMAC-SHA256 hash chain for crew and task events.
    agnosai owns this rather than delegating across the seam, and not by preference: hoosh's
    `/v1/audit` is **GET-only** and its `audit_record` has **no metadata slot**, so every agnosai
    event's payload would be discarded. That is parity, not divergence — in the Rust build the two
    chains were already separate instances with separate keys.
    **One deliberate fix to the Cyrius hoosh**, which resets `expect_prev` to genesis
    unconditionally in `verify` and so reports any evicted chain invalid forever. The Rust oracle
    carries a `first_valid_hash` forward on eviction; that is what is ported, and a test drives the
    ring past its cap to prove the chain still verifies. Where the two hoosh implementations
    disagree, the Rust one is the oracle.
    The entry hash uses the oracle's **sorted** key order (`BTreeMap`), not declaration order —
    getting that wrong yields a chain that verifies against itself and nothing else.
  - **llm_hoosh** — speaks hoosh **2.6.0**: `usage.cost_micro_usd`, `usage.provider`, and the
    `X-Hoosh-Cache` response header, surfaced as `agnosai_inference_response_cost_micro_usd`,
    `_provider` and `_cache`.
    **This deletes a planned module.** M5 bite 15b was going to port hoosh's pricing table into
    `src/llm_pricing.cyr` — 16 rows, per-provider fallbacks and a truncating cost expression copied
    verbatim so the numbers would reconcile against `/v1/costs`. A copy of another project's price
    list, guaranteed to drift the first time hoosh changed one. The gateway now reports the figure
    it already computed, so the port reads it instead. Filed as a hoosh issue on 2026-07-29, fixed
    upstream in 2.6.0, consumed here the same day.
    Two representation decisions that a caller can get wrong:
    an absent cost is **`AGNOSAI_NO_LIMIT` (-1), never 0** — a local model served through Ollama is
    genuinely free, so treating 0 as "unknown" would under-report every local inference; and
    `AGNOSAI_HOOSH_CACHE_UNKNOWN` is **distinct from `_MISS`** — "this was a real inference" and
    "this gateway is too old to tell me" are different facts, and only the first justifies billing.
    An unrecognised header value, including a cache tier a future hoosh might add, reads as UNKNOWN
    rather than MISS, so a new tier can never be mistaken for billable work.
  - **server_prompt_guard** — **pulled forward from M6**, the third module to come across that way
    after `server_ssrf` and `server_sse`. It is a hard blocker for `execute_task`, which calls it
    five times: `wrap_system_prompt` once and `sanitize` four times (context, task_description,
    expected_output, failed_output). 31 injection patterns, matched ASCII-case-blind over raw
    bytes, first match in table order wins.
    Output is **byte-exact**, and the reason is new since the oracle was written: hoosh caches
    responses server-side keyed on a hash of the whole request body, so this module's wrappers are
    now part of the cache key and a single byte of drift silently drops the hit rate to zero
    rather than failing anything. Every literal is pinned by a full-string assertion.
    **The port cannot panic where the oracle can.** The oracle truncates with
    `&text[..MAX_INPUT_LENGTH]`, and slicing a Rust `str` off a UTF-8 boundary panics — reachable
    from a user-supplied task description longer than 50,000 bytes whose 50,000th byte lands
    mid-codepoint. Cyrius Strs are byte slices, so the port truncates and carries on with one
    mangled character. A test covers exactly that input.
    Two byte-oriented bypasses are reproduced rather than fixed, because the oracle's own tests pin
    them as expected: a zero-width space or a newline between the words of a pattern defeats it.
  - **orch_crew_runner** (bite 1 of 3) — the module's **pure leaves**:
    `pick_best_agent`, `infer_provider`, `build_system_prompt`,
    `strip_provider_prefix`, `select_model`. crew_runner is 1468 lines, the largest in the port, so
    it goes in verified bites: `execute_task` next, then the runner struct and the three process
    modes.
    **Two of the seven leaves are deliberately not written.** `topological_sort` is a one-line
    delegation to the scheduler, which the port already has as `agnosai_topological_sort_tasks` —
    a second sorter would be two implementations of one algorithm that drift.
    `mood_adjusted_temperature` is unreachable: it needs a bhava `PersonalityProfile`, and the
    ported `AgentDefinition` has no personality field at all, so porting the f64 clamp arithmetic
    now would mean code no caller can reach and no test can honestly exercise.
    `pick_best_agent` **reuses `agnosai_rank_agents`** rather than scoring again — `rust-old`'s own
    benchmark benchmarks it as `rank_agents(...).first()`, which is both the licence and the proof.
    One visible consequence, recorded because sakshi is an audit trail here: on a malformed
    `required_tools` the oracle emits one warning per agent and this emits exactly one, because
    `rank_agents` hoists the extraction the oracle repeats per agent.
    Three provider vocabularies and two complexity vocabularies are documented as **deliberately
    different lists** in the module header, because unifying any pair is a silent divergence:
    `deepseek` is one word as a model needle and a strip segment but `deep_seek` on the wire, and
    an agent with `complexity = "simple"` correctly routes as Simple for model selection while
    scoring as medium for agent selection.
  - **orch_durable_state** — one pretty-printed JSON snapshot per crew at
    `{base_dir}/{crew_id}.json`, plus the `CrewState` (de)serialisers.
    `bayan_json_v_build_pretty(v, 2)` turns out to be **byte-identical** to
    `serde_json::to_vec_pretty` for this shape — 95 bytes for the oracle's own sample, 2-space
    indent, no trailing newline — so the snapshot format is exact rather than approximated.
    **Not built on patra**, contrary to the port plan's `durable_state (→ patra)`: patra is a full
    embedded SQL database over its own paged format with no export verb, and its `jsonl` mode is
    append-only, so it cannot produce the overwritable readable file the oracle's own tests assert.
    Plan line corrected; **no M5 module touches patra.** `StateStore` is not ported either — it is
    RPITIT, so not even `dyn`-compatible, with one implementor and zero callers tree-wide.
    Three stdlib gaps were filled locally: a recursive `mkdir -p` (the stdlib has only single-level
    `sys_mkdir`), a whole-file read that keeps `Err(EISDIR)` distinct from `Ok(vec![])` (which
    `file_read_all` collapses to 0, along with silently truncating), and a portable `mkdir` wrapper
    (`sys_mkdir`'s second argument is the mode on Linux and the path length on agnos — same name,
    and it compiles either way). Both stdlib gaps filed upstream.
    Two deliberate divergences, both tested: `crew_id` is validated
    ([ADR 008](docs/adr/008-durable-state-crew-id-validation.md)) because the oracle validates
    nothing and `save`/`load` are otherwise an arbitrary write and an arbitrary read for a caller
    passing a wire-supplied id; and `agnosai_deserialize_crew_state` requires `status` and
    `results`, which `serde` does and `agnosai_crew_state_from_value` does not — it defaults a
    missing `status` to `Pending` and maps an unknown spelling to `Pending`, silently rewriting a
    corrupt snapshot into a plausible one.
  - **server_sse** (`CrewEvent`, `EventBus`) — **pulled forward from M6**, the way `server/ssrf`
    was for M4: `crew_runner` holds a `broadcast::Sender<CrewEvent>` and `orchestrator` holds an
    `EventBus`, so neither of M5's two largest modules compiles without them. `event_stream`,
    which wraps a receiver in an axum SSE response, stays with M6.
    A `broadcast` channel becomes a **vec of per-subscriber channels**, the same forced shape
    pubsub took, since Cyrius channels are single-consumer. Three tokio behaviours a port loses
    silently are pinned by tests: a receiver starts at the ring's **tail** (events sent before it
    subscribed are not replayed), a send with **no receivers drops the event** rather than
    buffering it for a future subscriber, and a send **never blocks** — a subscriber that has
    stopped reading loses its oldest events. The last of those is why the receiver is a struct
    rather than a bare channel: it carries the lag count that tokio reports as `Lagged(n)`, and
    `agnosai_event_sub_lagged` takes-and-clears to match that error being delivered once.
    `drop(rx)` becomes `agnosai_event_sender_unsubscribe`, without which `cleanup_orphans` would
    be permanently inert.
- **chan_lossy** — **this closes port-plan blocker #4.** `agnosai_chan_push_lossy` gives
  `tokio::sync::broadcast::Sender::send`'s three properties that a blocking `chan_send` lacks: it
  never blocks, never fails for lack of room, and evicts the *oldest* entry when the ring is full.
  A 1:1 port of `tx.send(...)` onto `chan_send` would have converted never-block-lossy into
  block-forever — a crew whose event stream has no live reader would wedge the thread publishing
  to it. Built from the public channel verbs (`chan_try_send` reports -2 on full, `chan_try_recv`
  discards the oldest) rather than reaching into the 56-byte channel layout, which two separate
  thread backends implement. `orch_pubsub` is the first consumer; `EventBus` will be the second.
- **guarded_fetch** — the shared implementation of
  [ADR 007](docs/adr/007-audit-redirect-revalidation.md): an HTTP fetch that re-runs
  `agnosai_is_safe_url` on **every redirect hop**, refuses an https→http downgrade, fails closed
  on a `Location` it cannot resolve confidently, and bounds the whole chain by one deadline
  rather than re-arming per hop. Extracted from `tools_builtin_security_audit` at the **second**
  instance rather than the third, deliberately: two copies of a security control drift, and the
  drift is silent. `tools_remote_registry` is the second consumer, and the ADR named it when it
  was written. The 200 security_audit assertions pass unchanged across the move.
- **tools_remote_registry** — `agnosai_fetch_package`: an SSRF-guarded GET with the oracle's
  10 MB cap and 30s timeout, returning size, content type and bytes.

  **The oracle's module doc overstates the oracle.** It claims the module "fetches `.agpkg` ZIP
  bundles or raw WASM modules … validates them, and registers the contained tools in the
  `ToolRegistry`". The file contains no ZIP reader, no WASM parser and no registration, and it is
  `pub mod` with zero consumers anywhere in the Rust tree. So this is a **complete** port of the
  unit, not one waiting on `.agpkg` and WASM support — those formats gate the unwritten half of a
  doc comment, not any behaviour that exists to be matched. The roadmap's "can only deliver a
  guarded fetch" was right about the outcome and wrong about the reason.

  Both of the oracle's size checks are reproduced — the `Content-Length` pre-check and the
  post-download check against what actually arrived — because the second exists precisely to catch
  a server that under-declares. `content_type` keeps `Option<String>` semantics: absent is 0,
  which stays distinct from an empty value. Two divergences, both documented in-module: `HTTP 404`
  rather than the oracle's `HTTP 404 Not Found`, since sandhi carries the code but not the reason
  phrase and inventing a reason table would be guessing at what the server sent; and the response
  cap is set one byte above the package limit so that an exactly-10 MB package — which the
  oracle accepts, its check being `>` — is not turned into a transport error by sandhi.
- **server_ssrf** — `agnosai_is_safe_url` / `agnosai_is_private_host` / `agnosai_is_private_ipv4`
  / `agnosai_is_private_ipv6`. **Pulled forward from M6** because two M4 modules gate on it
  (`builtin/load_testing.rs` and `remote_registry.rs` both guard their outbound request with it).
  Hardened past a literal reading of the oracle: Rust gets octal / hex / short-form /
  decimal-integer host normalisation free from the `url` crate's WHATWG parser, so
  `http://0177.0.0.1/`, `http://0x7f000001/`, `http://2130706433/` and `http://127.1/` all resolve
  to loopback before the private-range check runs. A naive dotted-quad port would have classified
  every one of those as a hostname and let them through — the classic SSRF bypass. The trigger for
  the permissive parse is an all-numeric final label, which RFC 1123 forbids in a real domain, and
  a host that must be an address literal but will not parse as one fails closed.
- **order** — `agnosai_sort` (iterative in-place heapsort) and `agnosai_select_nth` (Hoare
  quickselect, median-of-3), plus `agnosai_percentile_i64`. **This closes port plan blocker #8**
  and a standing Phase 0 gate. Rust had `sort_unstable` / `select_nth_unstable`; Cyrius's stdlib
  has neither, and the plan measured an O(n^2) insertion sort over agnosai's 100k-entry
  percentile vector at 52.6 s.
- **llm** (M3, Phase 2) — **complete**. `src/llm.cyr` hub plus three submodules:
  - **llm_router** — `agnosai_route` / `agnosai_default_model` / `agnosai_parse_complexity`:
    task-complexity model routing over ModelTier, TaskType, Complexity and TaskProfile.
  - **llm_retry** — `agnosai_with_retry` / `agnosai_compute_delay` / `agnosai_is_retryable`:
    exponential backoff with jitter, and the retryable-error classifier. The oracle's async
    generic `with_retry` becomes a function pointer plus opaque context with a zero-allocation
    result contract — Cyrius has no futures, and a tagged `Result` per attempt would allocate on
    the bump allocator on the hot inference path.
  - **llm_hoosh** — the hoosh seam client. Replaces the oracle's `pub use hoosh::…` facade, which
    cannot survive the port: hoosh in Cyrius is a binary with no `dist/`, so it is consumed as a
    remote HTTP seam (per `thoth/src/hoosh.cyr`). Defines the inference types agnosai consumes
    locally (`agnosai_chat_*`, `agnosai_inference_request_*`, `agnosai_inference_response_*`,
    ProviderType), builds the OpenAI-compatible request body, extracts content / deltas / usage /
    errors / the model catalogue, frames SSE `data:` lines, and does the round trip over sandhi.
    One I/O call (`agnosai_hoosh_chat`); everything else is pure and unit-tested offline.
- **scripts/stack.sh** — brings up the AGNOS services agnosai needs for live testing (today hoosh
  only; daimon and bote hooks marked for M4/M6) and `check` drives the live round trip. Modelled
  on `thoth/scripts/stack.sh`, sharing the same `$STACK_HOME` so both can drive one gateway.
- **tests/smcyr/llm_live.smcyr** — the M3 exit check: a live chat-completion round trip through
  `agnosai_hoosh_chat`, not curl, so it exercises agnosai's own client. SKIPs (exit 0) when no
  gateway is reachable; fails loudly when one answers and the exchange is wrong.
- **units** — the shared `AGNOSAI_NS_PER_SEC` / `_MS` / `_US` time constants. No oracle module;
  extracted once `learning_profile` and `core_error` had each grown their own — under two
  different names for the same value — and `llm_retry` would have been the third consumer.
- **id** — `agnosai_uuid_*`: UUID v4 and v5 (RFC 4122) over the stdlib's kernel-entropy
  `random_bytes` plus `sha1`. This closes a Phase 0 scaffold gate. It exists rather than being a
  dependency because **mneme is unusable** — no lib block, no `dist/` bundle, and AGPL-3.0 against
  agnosai's GPL-3.0-only. No mneme text was copied. Verified against the published
  v5(DNS, "www.example.com") vector.
- **core** (M2, Phase 1) — **complete**. `src/core.cyr` hub plus all six oracle submodules:
  - **core_error** — `agnosai_error_*`: all 15 `AgnosaiError` variants with byte-exact `Display`
    parity, including a reimplementation of Rust's `Duration` `Debug` rendering for `Timeout`
    (`30s`, `1.5s`, `100ms`, `1.5µs`, `100ns`).
  - **core_message** — `agnosai_message_*`: `Message` and `MessageTarget` over bayan's JSON value
    tree, with serde's externally-tagged snake_case wire forms and declaration-order fields.
    Timestamps reproduce chrono's serde output exactly (RFC3339, `SecondsFormat::AutoSi` — 0, 3, 6
    or 9 fractional digits), which `lib/chrono.cyr`'s second-precision `iso8601` cannot do alone;
    without it a `Utc::now()` timestamp would not survive a round trip.
  - **core_task** — `agnosai_task_*`: `Task`, `TaskResult`, `TaskDAG`, `ProcessMode` and the three
    enums. `TaskPriority` keeps the oracle's explicit discriminants because it derives `Ord`, so
    the tier ordering is observable behaviour rather than an implementation detail.
  - **core_resource** — `agnosai_hw_*` / `agnosai_resource_budget_*` / `agnosai_accelerator_*`:
    devices, requirements, inventories, placement matching and budgets.
  - **core_agent** — `agnosai_agent_*`: `AgentDefinition` and `AgentState`, including the
    `hardware_requirement()` precedence rule (explicit `hardware` field over the legacy GPU
    fields, with `gpu_memory_min_mb` only applying when `gpu_required` is set).
  - **core_crew** — `agnosai_crew_*`: `CrewSpec`, `CrewState`, `CrewProfile`, `CrewStatus`.
  - **core_json** — shared JSON field helpers. No oracle module; extracted once task, resource and
    agent all needed the same `Option`/map/bool plumbing that serde's derives had provided.
- **id** — `agnosai_uuid_*` (listed above) also unblocked `core_message`, `core_task` and
  `core_crew`, all of which key on `Uuid`.
- **scripts/bench-history.sh** — rewritten to drive `cyrius bench` and append to a fresh root
  `bench-history.csv`. `cyrius bench` already handles discovery, timing and unit formatting, so
  only the unit normalisation and date/version stamping remain — the criterion name/timing
  pairing logic is gone. The Rust-era harness is preserved verbatim at
  `rust-old/scripts/bench-history.sh`.
- **learning** — the first real Cyrius code of the port (M2 beachhead, Phase 1). All five
  submodules of `rust-old/src/learning/` ported, with `src/learning.cyr` as the hub mirroring
  `mod.rs`:
  - **learning_capability** — `agnosai_capability_scorer_*`: confidence scoring over a Str-keyed
    map, with the bounded 64-observation recent window and the 5-observation trend verdict.
  - **learning_profile** — `agnosai_profile_*`: per-agent success rates and durations, with the
    10,000-record-per-agent eviction cap.
  - **learning_strategy** — `agnosai_ucb1_*`: UCB1 bandit, `mean + sqrt(2 * ln(N) / n)` over the
    `f64_sqrt` / `f64_ln` builtins.
  - **learning_replay** — `agnosai_replay_buffer_*`: prioritized experience replay sampling over
    tyche's `rng_uniform`, with lowest-priority eviction and the zero/NaN fallback path.
  - **learning_optimizer** — `agnosai_qlearner_*`: tabular Q-learning over a u64-keyed table with
    an interned-string front end.
- **tests** — 112 assertions across five `.tcyr` suites, covering all 35 `#[cfg(test)]` tests of
  the oracle plus branches the oracle never reaches: the UCB1 formula itself, the `max_by`
  tie rule, replay's zero-priority and NaN fallbacks, and the Q-table's packed-key distinctness.
  `cyrius coverage` reports 100% reference coverage (56/56 fns), against the 80% gate.
- **benches/learning.bcyr** — the 10 benchmark shapes of `rust-old/benches/learning.rs`, starting
  the Cyrius baseline. Not comparable to the frozen Rust CSV.

### Performance
- **server_auth**: the full auth decision — `Bearer ` extraction, the visible-ASCII
  gate over the whole header, two SHA-256 digests and a 32-byte constant-time
  compare — is `auth_check_secret_ok` **945 ns**. The disabled short-circuit is
  `auth_check_disabled` **6 ns**, so a deployment that has not configured auth pays
  essentially nothing.

  **The three secret-compare rows are the evidence for [ADR 009](docs/adr/009-auth-constant-time-secret-compare.md),
  not padding**: accept is 945 ns, reject 955 ns, and a 1-byte token against the
  same 9-byte secret 917 ns — within 4% of each other. The oracle's
  `max(a.len(), b.len())` loop would have made that last row track the *secret's*
  length, which is the leak the ADR closes. The residual ~28 ns spread tracks the
  **token's** length, which the attacker chose and already knows.
- **order**: the 100k-entry percentile workload from `builtin/load_testing.rs`, against the
  52.6 s O(n^2) baseline the port plan measured — `sort_100k_heapsort` **78.1 ms** (~670x),
  `three_percentiles_100k` **10.6 ms** (~5,000x). Both beat the plan's predictions (87 ms and
  ~21 ms respectively), and each timed round includes a full 100k copy, so the algorithms alone
  are faster still. The adversarial guards hold: `sort_100k_already_sorted` is 77.7 ms, the same
  as random input (heapsort has no worst case), and `select_nth_100k_already_sorted` is 4.12 ms,
  *faster* than random — median-of-3 keeps sorted input off quickselect's O(n^2) path.
- **tools_builtin_load_testing**: the post-join aggregation at the 100k request cap is
  `lt_aggregate_100k_10workers` **79.2 ms**, of which the 74.6 ms sort is nearly all of it — the
  cross-worker merge costs ~5 ms. Spreading the same 100k samples over 500 workers instead of 10
  costs 2.3 ms more (**81.5 ms**), confirming the merge is O(total) rather than O(workers), which
  matters because each worker owns an arena freed with it, so aggregate must copy rather than
  alias.
- **tools_builtin_load_testing**: `lt_aggregate_100k_200codes` **149 ms** puts a number on the
  decision to keep status counts in a linear pair vec rather than a map. 200 distinct codes cost
  1.9x — but HTTP defines ~60 codes and a real run sees one to five, so this row is the documented
  ceiling rather than an expected cost.
- **server_ssrf**: `is_safe_url_public_host` **1.30 µs**, `is_safe_url_octal_host` **1.38 µs**.
  Every load test pays one of these before it touches the network; the numeric-host path, which
  must try four different address spellings before it can decide, costs only 6% more than a plain
  hostname.
- **tools**: `tool_registry_get` **467 ns** (hashmap hit behind the futex mutex),
  `tool_execute_echo` **973 ns** for a full call through the vtable including input construction.
- **tools_agnos**: `agnos_segment_ok` **281 ns** and `agnos_form_encode_52b` **1.15 µs** over a
  52-byte query. Both are per-call costs on the ecosystem tools, and both are dwarfed by the
  network exchange that follows them.
- **orch** (new `benches/orch.bcyr`, first numbers for the orchestration group):
  `conv_buffer_push_sliding_32` **120 ns**, `pattern_match_wildcard` **803 ns**,
  `event_round_trip_1_sub` **1.99 µs**, `plan_cache_get_hit` **2.11 µs**,
  `pubsub_publish_4_patterns` **5.72 µs**, `plan_key_16x16` **11.7 µs**, `rank_agents_16`
  **12.8 µs**, `kahn_sort_64_nodes` **57.6 µs**, `event_fanout_64_subs` **101 µs**,
  `delegate_16_tasks_16_agents` **204 µs**.
  **These are futex-bound, not algorithm-bound**, and finding that out is what the benchmarks
  bought: `mutex_lock` + `mutex_unlock` costs **394 ns uncontended**, because `lib/sync.cyr`'s
  two-state mutex issues `FUTEX_WAKE` on every release whether or not a waiter is parked. A
  scratch build with that single line deleted measures **46 ns** — an 8.6× gap — and
  `chan_try_send` + `chan_try_recv` at **1.59 µs** is about four mutex pairs. So
  `event_round_trip_1_sub` is three locks and almost nothing else. Filed upstream in the cyrius
  repo as `docs/development/issues/2026-07-29-mutex-unlock-unconditional-futex-wake.md` with a
  repro; nothing is worked around here, so a stdlib fix lands as a straight improvement.
- **orch_audit**: `audit_record` **27.9 µs** (a uuid, a JSON build, a SHA-256 and an HMAC-SHA256
  per entry — one per sequential task), `audit_verify_256` **2.65 ms** for a full 256-entry chain,
  which re-hashes and re-signs every entry and is an auditor-triggered operation rather than a hot
  path.
- **server_prompt_guard**: `prompt_scan_clean_67b` **8.04 µs**, `prompt_scan_clean_4k`
  **555 µs → 273 µs (−51%)**. The scan is O(len × 31 patterns) and `execute_task` runs it on up to
  four fields per task, so the naive form cost ~27 ms of CPU for a task at the 50,000-byte field
  cap. Hoisting a first-byte guard — compare the needle's folded first byte before entering the
  inner comparison — halves it with nothing observable changed. Deliberately not optimised further;
  the module header records why, and what the next step would be if it ever matters.
- **orch_crew_runner**: `crew_select_model_routed` **301 ns** (the non-override path, so it pays
  `parse_complexity` plus the routing matrix on every task), `crew_infer_provider_fallthrough`
  **594 ns** (the worst case — a lowercase fold plus all nine anchored prefix tests),
  `crew_build_system_prompt` **1.47 µs** for a fully-populated agent, the only allocating leaf.
- **orch_durable_state**: `durable_serialize_crew_state` **3.88 µs**,
  `durable_deserialize_crew_state` **2.68 µs**, `durable_load_hit` **4.43 µs**,
  `durable_load_miss` **2.80 µs**, `durable_save_atomic` **21.5 µs** (fsync-bound by construction —
  `file_write_atomic` writes a temp, fsyncs, renames).
- **orch_durable_state**: `durable_mkdir_p_existing_4deep` **43.0 µs → 6.0 µs (−86%)**, and
  `durable_save_atomic` **39.1 µs → 21.5 µs (−45%)** with it, since every save pays one. The first
  implementation walked every path component unconditionally — two syscalls each, even when the whole
  tree already existed, which made the directory check cost more than the fsync'd write after it.
  `std::fs::create_dir_all` tries `mkdir` on the **full path first** and only walks parents on
  `ENOENT`; adopting that ordering is both faster and closer to the oracle.
- **server_sse**: `event_send_evicting` **2.40 µs** against `event_round_trip_1_sub` **1.99 µs**
  is the number blocker #4 was about — a subscriber that has stopped reading costs **one extra
  channel operation** to serve, not an unbounded stall. It loses events instead of wedging the
  crew publishing to it, which a blocking `chan_send` would have done.

### Changed
- **Money is integer micro-USD** across the cost path (`ResourceBudget::max_cost_usd`,
  `CrewProfile::cost_usd` and both per-key cost breakdowns), per the 2026-07-28 decision.
  Amounts accumulate exactly with no float drift. Serialization still routes through an f64
  under the original `*_usd` wire names, so bayan's Grisu2 emits the shortest round-tripping
  form — `0.0025`, `5.0`, `0.000001` — byte-identical to serde. **This corrects the port plan's
  prediction** that micro-USD would cost a `0.002500` vs `0.0025` textual divergence: converting
  only at the wire boundary avoids it entirely.
- **core** — divergences from the Rust API, each documented at the top of its module:
  `Option<T>` numeric fields use a `-1` sentinel (`AGNOSAI_NO_LIMIT`); `Option<String>` fields use
  `0`; `from_json` returns `0` in place of `Result::Err`; `HashMap<Uuid, _>` keys become the
  canonical UUID string, which is what serde emitted. Every `skip_serializing_if` in the oracle is
  honoured exactly, since those omissions are wire-visible.
- **core_agent** — `personality` is not ported (bhava is post-v2), but the field still serializes
  as `"personality": null`, which is what the default Rust build emitted. Incoming values are
  accepted and ignored. This resolves port plan open question 2 the conservative way, preserving
  byte-exact default-build wire parity; say so if you want the field dropped from the wire instead.
- **core_resource** — the `#[cfg(feature = "hwaccel")]` half is not ported (ai-hwaccel re-exports,
  `TrainingMemoryEstimate`, the `detect`/`from_hwaccel` probes). `hwaccel` is not in the default
  build, so it sits outside the v2.0.0 parity bar alongside bhava. 19 of the oracle's 28
  resource tests port; the 9 hwaccel-gated ones defer with the feature.
- **learning** — three deliberate shape divergences from the Rust API, each documented at the top
  of its module. `Option<T>` returns become presence-return plus an out-param, keeping the query
  paths allocation-free where a tagged `Option` would heap-allocate. `Duration` becomes an i64
  nanosecond count and `DateTime<Utc>` becomes epoch nanoseconds. `CapabilityScorer::all_scores`
  splits into `agnosai_capability_scorer_keys` + `_score`, which together cover the same surface
  without minting a pair per entry. Wire behaviour is unchanged; `learning` has no consumers
  outside itself (verified by the port plan's grep), so no downstream code is affected.
- **tests/agnosai.tcyr**: replaced the stock `proj-tcyr` epilogue with the clamp-safe form. The
  stock epilogue passes the raw failure count to `exit`, which the kernel masks `& 0xFF` — so
  exactly 256, 512 or 768 failures would have scored as PASS.

### Fixed
- **orchestrator/crew_runner**: `cargo check --no-default-features --features kavach` failed to
  compile. The `sandbox_strength` block was gated on `kavach` alone but reaches into
  `crate::sandbox::`, which `lib.rs:27` gates on `sandbox`; only the `full` feature (which enables
  both) hid the breakage. Gate is now `all(feature = "kavach", feature = "sandbox")`. This was
  blocker #7 of the Cyrius port plan — the Rust tree must be green before it can serve as the
  port's parity oracle.
- **server/ssrf**: collapsed the private-IP `match` into a single boolean so the IPv4 and IPv6
  arms share one return path (clears `clippy::collapsible_match` on Rust 1.96).
- **orchestrator/scheduler**: `ready_tasks` sorts with `sort_by_key(Reverse(priority))` instead of
  a hand-written comparator (clears `clippy::unnecessary_sort_by` on Rust 1.96). Ordering is
  unchanged — both are stable sorts, highest priority first.

### Changed
- Final Rust release line before the Cyrius port. `bench-history.csv` is frozen at this point and
  moves to `rust-old/`; the Cyrius tree starts a fresh baseline. tokio-era numbers are **not**
  comparable across the port — see `docs/development/cyrius-port-plan.md`.

## [1.1.0] — 2026-04-02

### Changed

#### License
- AGPL-3.0-only → GPL-3.0-only

#### Dependencies
- bhava: 1.8.0 → 2.0.0
- bote: local path dep → 0.91.0 from crates.io
- serde_yaml 0.9 (deprecated) → serde_yaml_ng 0.10 (maintained fork)

#### Infrastructure
- **scripts/version-bump.sh**: now updates all crate Cargo.tomls (root, SDK, examples) from VERSION
- **supply-chain/config.toml**: added publisher trust for 12 maintainers/orgs, pruned stale exemptions
- Version parity enforced: agnosai, agnosai-tool-sdk, and hello-tool all track VERSION

### Fixed
- **fleet/registry**: `usize → u32` GPU count cast now uses `try_from` with saturation instead of silent truncation
- **orchestrator/budget**: cost rounding uses `ceil()` to prevent budget underflow from floating-point truncation
- **llm/retry**: removed dead `last_err` variable and replaced `.expect("unreachable")` with `unreachable!()` for clarity
- **sandbox/wasm**: `extract_exit_code` now uses debug formatting to match fuel/epoch keywords in wasmtime error chain
- **orchestrator/ipc**: corrected misleading TOCTOU comment on socket bind
- **benches/server**: added missing `definitions` field to `AppState` construction
- **scripts/bench-history.sh**: fixed median extraction parsing — was capturing unit suffix instead of value, causing all CSV entries to be 0
- **docs**: updated stale version refs, test counts, benchmark dates, scoring weights, and serde_yaml→serde_yaml_ng across 8 doc files

### Observability
- **orchestrator/output_validation**: `warn!()` on all validation failure paths (invalid JSON, type mismatch, missing fields)
- **orchestrator/crew_runner**: `debug!()` logging in `pick_best_agent()` with agent key, score, and task ID
- **sandbox**: `#[tracing::instrument]` on `manager::execute_argv`, `process::execute_argv`, `python::execute_script`, `oci::execute`

### Performance
- `#[inline]` on 8 hot-path accessors: `BudgetTracker::{tokens_used, cost_usd, has_limits}`, `ApprovalGate::pending_count`, `Scheduler::{len, is_empty}`, `PubSub::pattern_count`

### Tests (863 total, up from 824)
- **LoadTestingTool**: +5 tests (mock server execution, error status codes, connection refused, SSRF gate, trait-level SSRF)
- **SecurityAuditTool**: +7 tests (missing headers, good headers, CORS wildcard+credentials, information disclosure, HTTPS recommendation, trait-level SSRF)
- **SSE EventBus**: +7 tests (orphan cleanup, capacity boundary, broadcast overflow/lagged, concurrent subscribers, crew isolation, sender idempotency, has/remove)
- **Prompt injection adversarial**: +10 tests (mixed case, buried injection, multiple patterns, empty input, unicode padding, newline splitting, all 30 patterns, boundary markers, anti-injection directive, exact max length)
- **WASM sandbox escape**: +4 tests (fuel exhaustion, epoch timeout, valid module execution, zero-length input)
- **Telemetry**: +6 tests (record_usage, all attr constants, OTel prefix, agnosai prefix, guard lifecycle, env filter)

## [1.0.2] — 2026-03-29

### Changed

#### Dependencies
- hoosh: 1.0.0 → 1.1.0

## [1.0.1] — 2026-03-28

### Changed

#### Dependencies
- majra: 1.0.1 → 1.0.2
- rustc-hash: 2.1.1 → 2.1.2 (transitive)
- zerocopy: 0.8.47 → 0.8.48 (transitive)
- zerocopy-derive: 0.8.47 → 0.8.48 (transitive)

## [1.0.0] — 2026-03-27

### Added

#### Security — Prompt Injection & Tool Allow-Lists
- **Prompt injection detection** (`server::prompt_guard`): heuristic scanner for 30+ injection patterns (instruction override, role hijack, prompt leak, delimiter injection) with case-insensitive matching
- **Input sanitization**: `sanitize()` truncates inputs to 50K chars, wraps in `<user_input>` boundary markers, logs warnings on suspicious content
- **System prompt hardening**: `wrap_system_prompt()` adds `<system_instructions>` delimiters and anti-injection directive to all LLM system prompts
- **Per-agent tool allow-list enforcement**: `ToolRegistry::get_allowed()` validates tool calls against agent's `tools` field before execution; empty list means "all tools"
- `ToolRegistry::is_tool_allowed()` static helper for allow-list checks

#### Structured Output Validation
- **Output validation module** (`orchestrator::output_validation`): validates LLM responses against JSON Schema (`type` and `required` field checks)
- **Retry-on-parse-failure**: when `Task.output_schema` is set, failed validation triggers up to 2 retries with error feedback injected into the prompt and temperature forced to 0.1
- **Markdown fence extraction**: `extract_and_validate()` automatically extracts JSON from ````json` code blocks in LLM responses
- `Task.output_schema` field — optional JSON Schema for output validation with retry

#### Human-in-the-Loop Approval Gates
- **Approval gate module** (`orchestrator::approval`): suspends crew runner via oneshot channels, resumes on HTTP callback
- `ApprovalGate` with configurable timeout (default 5 min), max 1,000 pending approvals, capacity enforcement
- `TaskRisk` enum (Low/Medium/High) on `Task` — determines whether human approval is required
- `ApprovalGate::requires_approval()` — configurable per-risk-level gating
- **REST endpoints**: `POST /api/v1/approvals` (submit decision), `GET /api/v1/approvals` (list pending)
- `ApprovalDecision` enum (Approved/Rejected) with serde support
- `AppState.approval_gate` — shared approval gate accessible from all route handlers

#### Kavach Integration (`kavach` feature flag)
- **`sandbox::kavach_bridge`** — bridge module mapping AgnosAI sandbox policies to kavach sandboxes
- `map_backend()` — maps `IsolationLevel` (None/Wasm/Process/Oci) to kavach `Backend`
- `build_config()` — converts `SandboxPolicy` to kavach `SandboxConfig` with externalization gate, seccomp, and agent ID
- `strength_for_policy()` — computes kavach `StrengthScore` (0–100) for any sandbox policy
- `execute()` — full lifecycle: create → start → exec → stop → destroy, with tracing and security metadata
- `scan_output()` — standalone externalization gate for scanning native tool outputs (secrets, code violations, PII)
- `policy_for_trust()` — maps crew trust levels ("minimal"/"strict"/"basic") to kavach `ExternalizationPolicy` presets
- `KavachToolResult` — result struct carrying output, exit code, strength score, and scan verdict
- kavach 1.0.1 as optional dependency with `process` feature (seccomp, Landlock, credential scanning)
- **Sandbox strength in crew metadata**: `CrewProfile.sandbox_strength` — kavach strength score (0–100) carried in crew execution results
- **Per-crew isolation policy**: `CrewSpec.trust_level` ("minimal"/"basic"/"strict") — controls externalization gate thresholds via `policy_for_trust()`

#### Resilience & Context (P1)
- **LLM inference retry with exponential backoff** (`llm::retry`): configurable `RetryConfig` (max retries, base delay, max delay, jitter), `with_retry()` async wrapper, `is_retryable()` heuristic for transient errors (rate limits, 503s, timeouts, connection resets), wired into crew runner inference path
- **Token/cost budget enforcement** (`orchestrator::budget`): `BudgetTracker` with atomic counters, `check()` validates before inference, `record_tokens()`/`record_cost()` after, `BudgetExceeded` error enum (Tokens/Cost variants)
- **Multi-turn conversation memory** (`orchestrator::memory`): `ConversationBuffer` with three strategies — `Full` (unlimited), `SlidingWindow` (evict oldest), `HeadTail` (keep first + last N), per-agent context accumulation
- **OTel GenAI semantic convention spans** (`telemetry::genai`): `inference_span()`, `tool_span()`, `crew_span()` helpers emitting standardized attributes per OTel v1.37 (`gen_ai.operation.name`, `gen_ai.agent.name`, `gen_ai.usage.input_tokens`, `gen_ai.response.model`, etc.)
- **Per-task cost attribution**: `CrewProfile.task_cost_usd` (per-task) and `CrewProfile.agent_cost_usd` (per-agent) cost breakdowns populated from TaskResult metadata

#### Majra Integration (`majra` feature flag)
- **Priority inference queue** (`llm::inference_queue`): `InferenceQueue` backed by majra's `ConcurrentPriorityQueue` — enqueue inference requests at 5 priority tiers (Critical→Background), async worker loop dispatches in priority order, oneshot reply channels
- `map_priority()` — maps AgnosAI `TaskPriority` to majra `Priority`
- `InferenceQueue::spawn_worker()` — background task that pops and executes queued inference requests
- **Per-endpoint rate limiting** (`server::rate_limit`): `RateLimitState` backed by majra's token bucket `RateLimiter` — per-IP rate limiting with X-Forwarded-For/X-Real-IP extraction, stale key eviction, HTTP 429 middleware
- majra 1.0.1 as optional dependency with `queue`, `ratelimit`, `pubsub` features

#### Medium-Priority Batch — Ecosystem, Observability, Durability
- **Topology-aware fleet scheduling** (`fleet::topology`): `NodeTopology`, `DeviceLink`, `InterconnectType` (PCIe/NVLink/XGMI/CXL), `topology_score()` for multi-GPU placement, `supports_tensor_parallel()` check
- **Cost-aware crew planning** (`fleet::cost_planning`): `GpuPricing` with per-hour rates for 5 GPU types, `estimate_crew_cost()`, `select_cheapest_model()` budget-constrained selection
- **Container/VM environment detection** (`fleet::environment`): `RuntimeEnvironment` enum (Bare/Container/Vm/Kubernetes/Unknown), `detect()` via cgroup/hypervisor/env inspection, `resource_limits()` from cgroup v1/v2
- **Multi-node fleet discovery** (`fleet::discovery`): `DiscoveryBackend` trait, `StaticDiscovery` impl, `DnsDiscovery` stub for DNS SRV
- **Prometheus metrics** (`server::prometheus`): `AgnosMetrics` with atomic counters (crews, tasks, tokens, cost), `gather()` in Prometheus exposition format
- **Multi-tenancy** (`orchestrator::multi_tenant`): `TenantRegistry` with DashMap, `TenantBudget`, per-tenant budget checking
- **Durable crew state** (`orchestrator::durable_state`): `StateStore` trait, `FileStateStore` impl (JSON to disk), `serialize_crew_state()` / `deserialize_crew_state()`
- **Hierarchical process mode** (`orchestrator::hierarchical`): `delegate_tasks()` using scoring module to assign tasks to best-fit agents, replaces sequential fallback
- **Sensitive information output filter** (`server::output_filter`): `OutputFilter` scanning for system prompt leakage, API keys (AWS/GitHub/Bearer), PII (email/phone/SSN), `scan()` + `redact()`

#### Low-Priority Batch
- **Hot-reload tool registration**: `DELETE /api/v1/tools/{name}` for runtime tool unregistration
- **Dashboard API**: `GET /api/v1/dashboard/crews` (crew history summaries), `GET /api/v1/dashboard/agents` (agent performance from recent runs)
- **Remote WASM tool registry** (`tools::remote_registry`): `fetch_package()` downloads tool packages from URL with SSRF protection and 10 MB size limit
- **Hot-reload configuration** (`server::hot_config`): `ConfigHolder<T>` backed by `tokio::sync::watch` — zero-contention reads, instant propagation, `RuntimeConfig` struct with reloadable settings
- **Plan caching** (`orchestrator::plan_cache`): `PlanCache` with LRU eviction (256 max), TTL expiry, order-independent crew spec hashing via `PlanKey`
- **Kubernetes CRD types** (`definitions::k8s_crd`): `CrewCrd`, `AgentCrdSpec`, `TaskCrdSpec` with `agnosai.io/v1` API group — serde-compatible, no k8s client dependency

#### Infrastructure
- **Graceful shutdown** in `main.rs` — handles SIGTERM and SIGINT via `tokio::signal`, logs shutdown reason
- **`scripts/bench-history.sh`** — runs all benchmarks and appends median times to `bench-history.csv`

### Changed
- `main.rs`: HTTP client build uses `?` instead of `.expect()` (no longer panics on TLS init failure)
- Crew runner: system prompts wrapped with anti-injection boundaries via `prompt_guard::wrap_system_prompt()`
- Crew runner: task descriptions and context values sanitized via `prompt_guard::sanitize()` before LLM submission
- Crew runner: output validation retry loop when `Task.output_schema` is set

#### `#[must_use]` additions (26 methods)
- `fleet::registry`: `get`, `list`, `list_online`, `count`, `count_online`, `find_by_capability`
- `fleet::placement`: `place`, `rank_nodes`
- `fleet::gpu`: `compute_devices`, `devices`, `devices_of_type`, `total_memory_mb`, `available_memory_mb`, `total_vram_mb`, `available_vram_mb`, `best_device`, `allocations`, `vram_available_mb`
- `fleet::state`: `get`, `active_runs`, `overall_progress`
- `fleet::coordinator`: `tasks_for_node`, `is_complete`, `completion_pct`, `pending_reassignment`, `state_manager`
- `definitions::versioning`: `get`, `latest`, `list_versions`

### Security
- **Prompt injection defence-in-depth**: boundary markers, anti-injection directives, heuristic scanning
- **Tool call allow-list**: prevents LLM from invoking tools outside agent's declared tool set
- **VersionStore bounded growth**: capped at 500 versions per agent with oldest-first eviction
- **Load testing tool request cap**: total requests capped at 100K across all concurrent users

### Fixed
- **Output validation retry prompt injection**: failed LLM outputs are now sanitized via `prompt_guard::sanitize()` before inclusion in retry prompts
- **Output validation retry prompt growth**: retry prompts now use the original prompt as base, preventing exponential accumulation
- **JSON fence extraction**: closing delimiter now requires newline prefix (`\n`````), preventing false termination on literal triple backticks inside JSON string values
- `#[non_exhaustive]` added to `WasmToolManifest`

### Performance
- **`rank_agents` (10 agents)**: 2.95 µs → 870 ns (−71%) — pre-extract `required_tools` once per task instead of re-deserializing per agent
- **Crew cancel/update**: O(n) → O(1) — `active_crews` changed from `Vec<CrewState>` to `HashMap<CrewId, CrewState>`
- **DAG topological sort**: O(n² log n) → O(n log n) — replaced Vec + sort() with BinaryHeap for priority ordering
- **`scan_input` prompt guard**: zero-alloc — replaced `to_ascii_lowercase()` with `eq_ignore_ascii_case` byte-window search
- **`rank_agents` scoring loop**: single `extract_required_tools()` call shared across all agents (was N calls)
- **Server endpoints**: GET /health −43%, POST /mcp −40%, EchoTool −37%

### Observability
- `llm::router::route()` — `tracing::debug` on model tier routing decisions
- `tools::builtin::load_testing` — `tracing::info` on test start/completion with metrics
- `learning::profile` — `tracing::debug` on action recording
- `learning::optimizer` — `tracing::debug` on Q-value updates
- `learning::capability` — `tracing::debug` on capability success/failure with confidence and trend

### Tests (823 total, up from 620)
- Prompt guard: 12 tests (injection patterns, sanitization, boundary wrapping)
- Output validation: 12 tests (JSON parsing, type checks, required fields, fence extraction, retry prompts, backtick edge case)
- Approval gate: 7 tests (approve/reject flow, timeout, capacity, cancel, listing)
- Tool allow-list: 5 tests (empty list, allow/block, missing tool)
- Kavach bridge: 16 tests (backend mapping, strength scoring, config building, externalization gate, trust policies)
- Crew trust/strength: 5 tests (default trust, custom trust, strength serialization, serde roundtrip)
- LLM retry: 11 tests (exponential backoff, retryable detection, transient recovery, exhaustion, non-retryable skip)
- Budget tracker: 6 tests (token/cost enforcement, accumulation, display)
- Conversation memory: 8 tests (full/sliding/head-tail strategies, clear, serde)
- GenAI spans: 4 tests (attribute naming, span creation)
- Inference queue: 4 tests (creation, priority mapping, enqueue, background)
- Rate limiter: 7 tests (burst, separate keys, stats, eviction, header extraction)
- Topology scheduling: 7 tests (single GPU, no links, full NVLink, partial, tensor parallel)
- Cost planning: 8 tests (estimation, model selection, budget constraints)
- Environment detection: 7 tests (enum variants, resource limits, detection)
- Fleet discovery: 7 tests (static backend, discovered node, trait impl)
- Prometheus metrics: 7 tests (counters, gauges, gather format)
- Multi-tenancy: 11 tests (registration, budget check, concurrent access)
- Durable state: 7 tests (serialize/deserialize, file store, tempdir)
- Hierarchical mode: 6 tests (task delegation, scoring-based assignment)
- Output filter: 16 tests (API keys, PII, system prompt leak, redaction)
- Plan cache: 7 tests (insert/get, order-independent hashing, TTL expiry, LRU eviction)
- K8s CRD types: 4 tests (serde roundtrip, YAML compat, defaults, API constants)
- Hot config: 5 tests (initial value, update, receiver, runtime config defaults, serde)
- Remote registry: 3 tests (SSRF rejection, localhost rejection, size constant)
- VersionStore eviction: 1 test

## [0.24.3] — 2026-03-24

### Added

#### Features
- **OpenTelemetry tracing spans** (`otel` feature flag): `src/telemetry.rs` with `init_tracing()`, OTLP gRPC export, `TracingGuard`, env var auto-detection (`OTEL_EXPORTER_OTLP_ENDPOINT`, `OTEL_SERVICE_NAME`)
- **`#[tracing::instrument]`** on `Orchestrator::run_crew`, `cancel_crew`, `CrewRunner::run/run_sequential/run_parallel/run_dag`, `execute_task`, `score_agent`, `create_crew`, `a2a::receive`, `mcp_handler`
- **Crew cancellation**: `cancel_crew()` stops running crews via `AtomicBool` token — sequential breaks between tasks, parallel aborts pre-semaphore, DAG halts between waves
- **Cryptographic audit chain**: HMAC-SHA256 tamper-proof event logging via `hoosh::audit::AuditChain` — records `crew_accepted`, `crew_finished`, `crew_cancelled`, `task_completed` with metadata
- **SSRF protection module**: `server::ssrf` shared utilities — `is_safe_url()`, `is_private_ip()`, `is_private_ipv4()` — used by A2A callbacks, `LoadTestingTool`, and `SecurityAuditTool`
- **Configurable parallel concurrency**: `CrewRunRequest.max_concurrency` field (default 4, clamped 1–64)
- Constructors/builders for `ComputeDevice`, `HardwareInventory`, `HardwareRequirement`, `TaskDAG`, `ResourceBudget`, `Experience`, `RelayMessage`, `PlacementRequest`, `TaskProfile`, `TeamMember`, `ToolInput`
- `#[non_exhaustive]` on 48+ public structs across core, server, sandbox, fleet, learning, definitions
- `#[must_use]` on 30+ pure functions across scoring, learning, SSE, pubsub, tools, router
- `#[inline]` on 20+ hot-path accessors

#### Tests (620 total, up from 323)
- SSRF validation: 27 tests (IPv4/IPv6/mapped/localhost/schemes/metadata)
- Crew validation: 8 tests (cycle detection, self-deps, DAG mode)
- Fleet federation: 8 tests (election, roles, eviction)
- Fleet registry: 7 tests (heartbeat, capability search, online filtering)
- Scheduler `topological_sort_tasks`: 7 tests (chain, diamond, cycle, priority)
- Route handlers: SSE (3), tools (3), agents (3), definitions (1)
- WASM tool: 8 tests (manifest serde, output parsing)
- Python tool: 10 tests (JSON protocol, error handling)
- Audit chain: 3 tests (lifecycle, per-task events, cancel event)
- Telemetry: 2 tests

#### Benchmarks (106 across 17 files, up from 85 across 9)
- New: `orchestrator` (5), `llm_router` (10), `definitions` (3), `audit` (7), `server` (6), `sandbox` (4), `ipc` (3), `fleet` (7)
- Extended: `scoring` (+3), `relay` (+2), `tools` (+5)

### Changed

#### Dependencies
- hoosh: 0.22.3 → 0.23.4, now from crates.io (was local path)
- bhava: 0.23.3 → 1.0.0, now from crates.io, **always-on** (no longer optional)
- ai-hwaccel: 0.21.3 → 0.23.3
- wasmtime/wasmtime-wasi: 42 → 43
- `personality` feature flag **removed** — bhava is a required dependency
- Scoring weights permanently personality-aware (tool: 0.35, complexity: 0.25, GPU: 0.10, domain: 0.15, personality: 0.15)
- GPL-3.0 / GPL-3.0-only added to `deny.toml` license allowlist

#### API
- `PubSub::subscribe()` → returns `Option` (capped at 10,000 patterns)
- `Ucb1::select()` / `best_arm()` → return `Option<usize>`
- `SandboxManager::execute()` replaced by `execute_argv()` (no shell interpretation)
- `OrchestratorState` → `pub(crate)` (internal state no longer leaked)
- `AgentDefinition.personality` always compiled (was `#[cfg(feature = "personality")]`)
- MCP server reports `CARGO_PKG_VERSION`
- Domain scoring uses case-insensitive comparison
- Duplicate `topological_sort` in crew_runner eliminated — delegates to `scheduler::topological_sort_tasks()`
- Shared `reqwest::Client` via `OnceLock` in synapse/mneme/delta tools (was per-instance)

### Security
- **Constant-time comparison**: length comparison uses full `usize` (was truncated to `u8`)
- **SSRF hardening**: IPv6 private ranges (fc00::/7, fe80::/10), IPv6-mapped IPv4 (::ffff:x.x.x.x), bracketed IPv6 URL parsing via `url::Host`
- **SSRF on tools**: `LoadTestingTool` and `SecurityAuditTool` reject private/internal target URLs
- **URL path traversal**: mneme `note_id`, delta `owner`/`repo`/`pipeline_id` reject `/` and `..`
- **A2A callback timeout**: 30s limit on fire-and-forget callbacks
- **Process sandbox env sanitization**: `execute()` strips `LD_PRELOAD`/`LD_LIBRARY_PATH`/`DYLD_*`
- **PubSub DoS**: subscription patterns capped at 10,000
- **EventBus DoS**: channel count monitored with orphan cleanup
- **Fleet unbounded growth**: `CrewStateManager` evicts at 1,000 completed runs, `FleetCoordinator` evicts at 10,000 completed tasks
- **Replay buffer**: fixed biased weighted sampling
- **StringInterner**: `checked_add` on u32 ID allocation

### Fixed
- **DAG priority ordering**: sort ascending for `pop()` (was inverted — highest priority processed last)
- **Double inference call**: streaming now emits captured response instead of re-invoking LLM
- **Wrong crew_id in streaming events**: was using task_id
- **JoinError in parallel/DAG**: synthesizes Failed `TaskResult` (was silently dropped)
- **`is_zero` NaN masking**: corrupt cost data now serialized (was silently dropped)
- **`ComputeDevice.memory_available_mb`**: kept in sync on allocate/release (was stale after first alloc)
- **`remove_node` barrier deadlock**: removing a node auto-satisfies pending barriers
- **`declare_coordinator` stale role**: resets all clusters to Follower before setting new coordinator
- **IPC TOCTOU race**: socket removal uses unconditional `remove_file` + ignore NotFound
- **SSE serialization failure**: emits error JSON instead of empty string
- **DAG failure propagation**: failed tasks no longer treated as completed dependencies

### Performance
- `complexity_level` / `parse_complexity` / `domain_score`: zero-alloc via `eq_ignore_ascii_case`
- `format!` → `write!` on system prompt construction, error messages, expected output
- `#[inline]` on scoring hot paths, tool accessors, fleet GPU methods
- `CapabilityScore::recent` bounded to 64 entries
- `PerformanceProfile` records bounded to 10,000 per agent
- DAG priority lookup: O(1) HashMap (was quadratic scan)
- Consolidated `use std::fmt::Write` import in crew_runner

## [0.22.3] — 2026-03-23

### Added
- `personality` feature flag — optional bhava integration for agent personality modeling
- `AgentDefinition.personality` field — attach a `PersonalityProfile` to any agent (feature-gated)
- `with_personality()` builder method on `AgentDefinition`
- `build_system_prompt()` injects personality behavioral disposition into system prompts when personality is set
- Mood-driven temperature adjustment — `mood_adjusted_temperature()` maps creativity/curiosity/precision/risk traits to inference temperature (0.1–1.5)
- Personality-aware agent scoring — `personality_score()` factors trait groups and specific trait levels into task-agent assignment (15% weight)
- Task context fields: `personality_group` and `personality_trait` for personality-based agent selection
- bhava 0.22.3 as optional dependency (15-trait personality system, mood vectors, sentiment analysis)

### Changed
- hoosh dependency updated to 0.22.3 (with sentiment analysis support)
- Scoring weights redistributed when `personality` feature enabled (tool: 0.35, complexity: 0.25, GPU: 0.10, domain: 0.15, personality: 0.15)
- `full` feature now includes `personality`

## [0.21.3] — 2026-03-21

### Added
- Lazy LLM provider initialisation — `HooshClient` created on first inference via `OnceLock`, not at server startup
- Crew execution profiling — `CrewProfile` on every `CrewState` with wall time and per-task `task_duration_ms` metadata
- Inference response caching — hoosh `ResponseCache` (TTL + LRU eviction) wired into `execute_task`, shared across crews
- Dockerfile (multi-stage build, `rust:1.89-bookworm` builder, `debian:bookworm-slim` runtime)
- `strip_provider_prefix()` — normalises LiteLLM-style `provider/model` identifiers for inference

### Changed
- hoosh dependency updated from 0.20 to 0.21.3
- `Orchestrator::with_llm_url()` replaces eager `with_llm(Arc<HooshClient>)` as primary init path
- Server startup no longer creates LLM client — deferred to first crew execution

## [0.20.3] — 2026-03-18

### Added

#### Core (`agnosai-core`)
- `AgentDefinition` with JSON/YAML deserialization, default complexity, GPU fields
- `Task` with priority (5-tier), status lifecycle, dependency tracking, context map
- `TaskDAG` with `ProcessMode` variants: Sequential, Parallel, DAG, Hierarchical
- `CrewSpec` and `CrewState` for crew lifecycle management
- `Message` with topic-based targeting (Agent, Topic, Broadcast)
- `ResourceBudget` with token, cost, duration, and concurrency limits
- `GpuDevice` type for VRAM tracking
- `AgnosaiError` enum (13 variants) with `thiserror` derives and `From` impls

#### Orchestrator (`agnosai-orchestrator`)
- `Orchestrator` with `Arc<RwLock<State>>`, delegates to `CrewRunner`
- `Scheduler` — priority queue (5-tier VecDeque) + DAG topological sort (Kahn's algorithm) with cycle detection
- `CrewRunner` — full crew lifecycle: Sequential, Parallel (semaphore-bounded), DAG (wave execution), Hierarchical (sequential fallback)
- Agent scoring — 4-factor weighted (tool coverage 0.40, complexity 0.30, GPU 0.15, domain 0.15)
- Topic pub/sub with wildcard matching (`*` one segment, `#` zero or more), broadcast channels, DashMap
- IPC — Unix socket server/client with length-prefixed framing (4-byte BE u32 + JSON payload, 16 MiB max)

#### LLM (`agnosai-llm`)
- `LlmProvider` trait with `infer()` and `list_models()`
- 8 providers: OpenAI, Anthropic (direct HTTP), Ollama, DeepSeek, Mistral, Groq, LM Studio, hoosh
- OpenAI-compatible providers delegate to `OpenAiProvider` via newtype pattern
- Model router — task-complexity scoring across 7 task types × 3 complexity levels → Fast/Capable/Premium
- `ProviderHealth` — 5-point ring buffer, 3 consecutive failures → unhealthy, one success resets
- `ResponseCache` — LRU with TTL expiration, deterministic cache keys
- `TokenBudget` — per-agent + global token accounting with `BudgetExceeded` errors
- `RateLimiter` — semaphore-based concurrent request limiting

#### Tools (`agnosai-tools`)
- `NativeTool` trait (object-safe with `Pin<Box<dyn Future>>`)
- `ToolRegistry` — thread-safe DashMap-backed tool storage
- Built-in tools: `EchoTool`, `JsonTransformTool`
- AGNOS ecosystem tools (optional HTTP clients, not hard dependencies):
  - Synapse: `synapse_infer`, `synapse_list_models`, `synapse_status`
  - Mneme: `mneme_search`, `mneme_get_note`, `mneme_create_note`
  - Delta: `delta_list_repos`, `delta_trigger_pipeline`, `delta_get_pipeline`

#### Sandbox (`agnosai-sandbox`)
- WASM sandbox via wasmtime — WASI preview 1, fuel-based CPU limits, epoch interruption for timeouts, memory caps, no filesystem/network
- Python subprocess bridge — stdin/stdout JSON protocol, `tokio::time::timeout`, kill-on-drop, tool wrapper script generation

#### Fleet (`agnosai-fleet`)
- `NodeRegistry` — node inventory with heartbeat TTL, status transitions (Online → Suspect → Offline), capability search
- `PlacementEngine` — 5 scheduling policies: GpuAffinity, Balanced, Locality, Cost, Manual
- `GpuScheduler` — device management, VRAM tracking, best-fit allocation/release
- `CrewStateManager` — distributed crew phases, barrier sync (per-node arrival tracking), named checkpoints, progress aggregation
- `FleetCoordinator` — task fan-out with node assignments, completion/failure tracking, retry with configurable max, reassignment

#### Learning (`agnosai-learning`)
- `PerformanceProfile` — per-agent action recording, success rates, duration averages
- `Ucb1` — multi-armed bandit strategy selection with UCB1 formula
- `ReplayBuffer` — prioritized experience replay with weighted sampling, lowest-priority eviction
- `CapabilityScorer` — dynamic confidence scoring with trend detection (Improving/Stable/Declining)
- `QLearner` — tabular Q-learning with configurable learning rate and discount factor

#### Definitions (`agnosai-definitions`)
- JSON/YAML loader — `load_from_file` (auto-detect), `load_all_from_dir`
- `assemble_team` — match `TeamMember` specs to agent definitions via role/tool/complexity scoring
- `VersionStore` — definition versioning with auto-incrementing version numbers, rollback

#### Server (`agnosai-server`)
- axum HTTP server with health/ready probes
- `POST /api/v1/crews` — create and execute a crew (index-based dependency mapping)
- `GET /api/v1/tools` — list registered tools
- Agent definition and preset placeholder endpoints
- `AppState` with shared `Orchestrator` + `ToolRegistry`

#### Documentation
- ADRs: separate repo, ecosystem tools, native HTTP providers, concurrency model
- Architecture overview with system context diagram and crate dependency graph
- Developer guides: getting started, adding providers, adding tools
- Contributing guide with commit conventions

#### Project
- Single-crate layout (edition 2024, MSRV 1.89)
- `rust-toolchain.toml` (stable channel)
- Release profile: `lto = "fat"`, `strip = true`, `panic = "abort"`, `codegen-units = 1`
- AGPL-3.0-only license
- 323 tests passing

#### Server — MCP, A2A, SSE, Auth
- MCP server (JSON-RPC 2.0 over HTTP POST): `initialize`, `tools/list`, `tools/call` with ToolRegistry integration
- A2A protocol: `POST /api/v1/a2a/receive` — webhook-based crew delegation with optional callback URL
- SSE streaming: `GET /api/v1/crews/:id/stream` — event stream endpoint with `CrewEvent` types
- Auth middleware: shared-secret Bearer token validation, configurable enable/disable

#### Definitions — Presets & Packaging
- 18 built-in presets (6 domains × 3 sizes): quality, software-engineering, devops, data-engineering, design, security
- `PresetSpec` type with `builtin_presets()`, `load_preset_from_json()`, `load_preset_from_file()`, `load_all_presets()`
- `.agpkg` ZIP packaging with decompression bomb protection (1 MiB per file, 100 entries max, path traversal rejection)
- `GET /api/v1/presets` returns built-in presets (feature-gated on `definitions`)

#### Auth — Full JWT (RS256)
- RS256 JWT validation with configurable issuer, audience, and expiry
- Constant-time shared-secret comparison (no timing or length leaks)
- Defense-in-depth: explicit `exp` claim requirement after decode
- Environment configuration: `AGNOSAI_AUTH_ENABLED`, `AGNOSAI_AUTH_SECRET`, `AGNOSAI_JWT_PUBLIC_KEY`
- `AuthConfig::with_secret()`, `AuthConfig::with_jwt()`, `JwtConfig::new()` builder methods

#### SSE — Full CrewRunner Integration
- `EventBus` with per-crew broadcast channels, lazy creation, orphan cleanup
- `CrewRunner` emits `crew_started`, `task_started`, `task_completed`, `crew_completed` events
- `Orchestrator` wires event bus to runners, cleans up channels on completion
- SSE endpoint handles lagged receivers (warns + notifies client)
- Unknown crew IDs return error event instead of leaking EventBus channels

#### Tools — Ported from Python + Community SDK
- `LoadTestingTool` — concurrent HTTP load generation with p50/p95/p99 latency, throughput, status codes
- `SecurityAuditTool` — HTTP header analysis, CORS detection, information disclosure, security scoring
- `agnosai-tool-sdk` crate for building WASM tools (ToolInput, ToolResult, run_tool)
- `wasm_loader` — load manifest.json + .wasm tool packages from directories
- Example WASM tool: `examples/wasm-tools/hello-tool/`

#### Agnostic Migration (Phase 5)
- Backend abstraction: `agents/backend/` package with `CrewBackend` trait, `CrewAIBackend`, `AgnosAIBackend`
- Feature flag: `AGNOSTIC_BACKEND=agnosai|crewai` routes crew execution
- Fleet shim: delegates fleet operations to AgnosAI via HTTP when backend is `agnosai`
- Docker Compose: `agnosai-server` service with `agnosai` and `e2e` profiles
- Dual-backend test infrastructure (unit + E2E)

### Changed
- License corrected from Apache-2.0 to AGPL-3.0-only in README
- Architecture diagram updated from fake workspace to actual single-crate structure
- Quick start commands and usage imports corrected
- `CONTRIBUTING.md` rewritten for actual single-crate structure
- Configurable server port via `PORT` / `AGNOSAI_PORT` env vars (was hardcoded 8080)

### Security
- Constant-time auth comparison (prevents timing attacks on shared secret)
- JWT expiry enforcement (defense-in-depth, rejects tokens without `exp` claim)
- `#[serde(deny_unknown_fields)]` on all API input types (TaskRequest, CrewRunRequest, A2ARequest, JsonRpcRequest)
- ZIP bomb protection in definition packaging (size limits, entry count, path traversal)
- NaN panic fix in replay buffer (`partial_cmp` with fallback)
- Load test duration clamped to 1-300 seconds (prevents division by zero)
- Environment sanitization: `LD_PRELOAD`, `LD_LIBRARY_PATH`, `DYLD_*` stripped from all sandboxed subprocesses
- Unbounded `active_crews` growth fixed (capped at 1000 with eviction)
- Concurrent crew limit enforced via semaphore (from `ResourceBudget.max_concurrent_tasks`)
- Crew execution timeout via `tokio::time::timeout` (from `ResourceBudget.max_duration_secs`)
- PubSub recursion depth limit (MAX_MATCH_DEPTH=32) prevents stack overflow
- IPC zero-length frame rejection
- IPC EOF vs truncated frame distinction in error messages
- Fleet barrier deadlock prevention (`force_barrier()`, `remove_node()`)
- Fleet checkpoint phase isolation (`is_checkpointing` flag)
- Fleet relay poisoned mutex recovery (resets seen-map instead of using corrupted data)
- A2A field validation (string length 10k, metadata 64 KiB)
- A2A DNS rebinding protection (blocks `.local`, `.internal`, `.localhost` suffixes)
- A2A shared HTTP client with 30s timeout (was creating new client per callback)
- Request concurrency limit (100 concurrent requests via tower)
- Scoring penalizes malformed `required_tools` context (0.5 instead of 1.0)
- Crew dependency cycle detection at API level (DFS before orchestrator)
- Error leakage prevention (internal errors logged at ERROR, generic message returned to client)

### Infrastructure
- `.gitignore`: `**/target/` (catches SDK/example build dirs), `.claude/`
- `SECURITY.md`, `CODE_OF_CONDUCT.md` added
- `deny.toml` for cargo-deny (license allowlist, ban wildcards, crates.io only)
- `supply-chain/config.toml` for cargo-vet (imports Mozilla audits)
- `docs/development/threat-model.md` — 6 attack surfaces mapped
- `docs/guides/adding-wasm-tools.md` — community SDK guide
- Fuzz testing: 4 targets (agent_definition, crew_request, preset_json, tool_input)
- CI: cargo-vet job, coverage job (≥55% gate), benchmark job, fuzz job (5 min), feature matrix testing, MSRV validation
- Makefile: `audit`, `deny`, `vet`, `bench`, `fuzz`, `coverage` targets
- `Cargo.toml`: `homepage`, `documentation`, `keywords`, `categories`, `exclude` fields
- Comprehensive structured logging (auth, orchestrator, crews, MCP, A2A)
- `#[must_use]` on `Result<T>` type alias
- Doc comments on all public types, enums, type aliases, and struct fields
