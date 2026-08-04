# Changelog

All notable changes to AgnosAI will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Security

- **Pinned kavach 3.11.1, which closes a Landlock hole agnosai found and fixed.**
  `security_apply_landlock` named only **3 of Landlock's 13 filesystem rights**
  in `handled_access_fs`, and Landlock permits every right it is not told to
  handle — so a process confined by kavach could still delete any file on the
  host, remove any directory, create directories anywhere, and exec any binary.

  **The smoke test that would normally catch this passed**, which is why it
  survived: reading was one of the three rights that *were* handled, so "a
  confined process cannot read `/etc/passwd`" — the exact escape test
  [ADR-006](docs/adr/006-cx-tool-sandbox.md) specifies for M7 — reported success
  while `unlink` was wide open. Found while planning M7's cx bites against that
  ADR, whose premise is that kavach's seccomp + Landlock *are* the entire
  security boundary for untrusted tool code.

  Measured before the fix, and again after, with the same probe: a process
  confined read-only to one scratch directory deleted `/tmp` files and
  directories outside it and created new ones. On 3.11.1 the victim file and
  directory survive. kavach's own regression test forks, confines the child for
  real, and is mutation-verified against the three-right mask.

### Added

- **M7 bite 7 — the spawn deadline: SIGKILL, reap, and an idle loop.** 132
  assertions in the suite; `agnosai_spawn_capture` and
  `agnosai_spawn_capture_input` both take a `timeout_ms`, and the result carries
  `timed_out` alongside the exit code and signal.

  **There is no `kill_on_drop` here.** The oracle gets teardown free from tokio
  (`process.rs:112`, `python.rs:76`), so every early return in Cyrius has to kill
  and reap by hand or leave a zombie holding the pipe ends open — after which the
  next drain waits for an EOF that is never coming.
  `_agnosai_spawn_kill_and_reap` is the one place that does it.

  **SIGKILL, not SIGTERM, and the test now depends on the difference.** The first
  version reported a hardcoded signal 9 on the timeout path, which made swapping
  in SIGTERM undetectable — every mutation passed. The reap now returns the raw
  wait status and the result reports the signal the child *actually* died from,
  and a child running `trap '' TERM; sleep 60` proves an ignorable signal is not
  enough.

  Five mutations, each caught: no reap (a zombie survives `waitpid(WNOHANG)`), no
  deadline check, no idle sleep, SIGTERM for SIGKILL, and `timed_out` pinned to 0.

### Fixed

- **The drain loop burned a full CPU core for as long as a child ran.** Both pipe
  read ends are `O_NONBLOCK`, so a read with nothing available returns
  immediately and the loop shipped in bites 5 and 6 simply retried. Measured
  against a child running `sleep 2`: **100% CPU for 2.006 s** — a server running
  sandboxed tools would have spent a core per concurrent tool doing nothing.

  An iteration that moves no bytes now sleeps 5 ms, which also supplies the tick
  the deadline is checked on. Same probe after the fix: **0% CPU**. The test
  compares the parent's own `CLOCK_PROCESS_CPUTIME_ID` against the wall time it
  waited, so removing the sleep fails rather than merely slowing things down.

- **A sandboxed child inherited the server's stdin.** `agnosai_spawn_capture` was
  a second copy of the drain loop that never touched fd 0, where the oracle pipes
  stdin on every spawn it makes (`process.rs:109`, `:189`). It is now defined as
  `agnosai_spawn_capture_input` with an empty input — one implementation, which
  cannot drift from itself the way the copy already had.

- **M7 bite 6 — `agnosai_spawn_capture_input`: the stdin feed.** 114 assertions
  in the suite. A 200,000-byte input round-trips through `cat` and comes back
  byte-for-byte.

  **This is a second deadlock, distinct from the stdout/stderr one, and the
  oracle's shape causes it.** `process.rs:133-139` and `python.rs:92-101` write
  the entire input before reading anything — which tokio survives only because
  its reader is a separate task. Reproduced literally on a blocking write, the
  parent fills the child's 64 KiB stdin pipe and blocks, while the child blocks
  on its own full stdout pipe that nobody is draining. Neither moves.

  So the write is non-blocking and takes its turn in the same loop as the two
  reads. Mutation-verified twice, both producing a **hang** rather than a wrong
  answer: writing the whole input first deadlocks the 200 KB case, and failing
  to close stdin on an *empty* input leaves `cat` waiting forever for an EOF
  that never comes.

- **M7 bite 5 — `agnosai_spawn_capture`: fork, three pipes, exec, status
  decode.** The primitive `process`, `python`, `oci`'s exec half and `manager`
  all sit on. 99 assertions total in the suite.

  **Spawn failure is now distinguishable from exit 127**, which is what the
  exec-errno pipe was built for and what three oracle sites require. A real
  program exiting 127 reports `SPAWN_OK` + code 127; `/nonexistent/binary`
  reports `SPAWN_FAILED`.

  **The drain interleaves, and the deadlock it avoids is real.** A child writing
  70,700 bytes to *each* of stdout and stderr — both past the 64 KiB pipe buffer
  — is captured whole. Mutation-verified: switching to drain-stdout-then-stderr
  makes the suite **hang**, because the child blocks writing stderr and so never
  closes stdout.

  **A note on how the CLOEXEC assertion got honest.** The first mutation —
  removing `FD_CLOEXEC` from the errno pipe — *passed all 97 tests*, because the
  errno pipe is read after the drain and EOF arrives anyway once the child
  exits. The flag only bites when a **grandchild** inherits the descriptor and
  outlives its parent: `sh -c 'sleep 30 &'` returns immediately but leaves
  `sleep` holding every inherited fd. With CLOEXEC the write end was closed at
  `execve` and the parent sees EOF at once; without it the parent blocks for 30
  seconds. With that case added, the mutation hangs the suite, which is the
  proof the first version lacked.

  A signalled child reports exit code **-1**, not 0 — matching the oracle's
  `status.code().unwrap_or(-1)`, where 0 would read as success — plus the signal
  number separately. stdout and stderr are captured **separately**, asserted in
  both directions so a port that merged them cannot pass.

- **M7 bite 4 — `sandbox/spawn`, sanitized `envp` + the CLOEXEC primitive.**
  77 assertions. No oracle file corresponds to this module: in Rust the work is
  `Command::env_remove` plus tokio's `Stdio` and `kill_on_drop`, spread across
  three call sites. Cyrius has none of that, so the three backends get one
  primitive underneath them rather than three hand-rolled `fork` loops.

  **The blocker that would have derailed this is resolved, and was proven before
  being designed in.** `fork` + `execve` cannot tell "spawn failed" from "the
  child ran and exited 127" — a failed `execve` leaves the child alive and every
  stdlib path resolves that to `sys_exit(127)`. Three oracle sites need the
  distinction (`oci.rs:219`, `process.rs:331`, `python.rs:87`). The answer is a
  fourth pipe whose write end carries `FD_CLOEXEC`, so the kernel closes it
  exactly when `execve` succeeds: bytes mean the exec failed, EOF means it
  worked. `pipe2` is not wrapped on x86_64, but `sys_pipe` +
  `syscall(SYS_FCNTL, wfd, F_SETFD, FD_CLOEXEC)` is. A probe forking
  `/bin/sh -c 'exit 127'` and `/nonexistent/binary` reported **spawned, exit
  127** for the first and **spawn failed** for the second.

  **Env sanitization matches on the NAME only** — everything before the first
  `=`. A variable whose *value* mentions `LD_PRELOAD` is not the variable
  `LD_PRELOAD`; testing the whole entry would drop innocent variables and, worse,
  would let attacker-supplied data steer which variables the filter removes.
  Mutation-verified: whole-entry matching fails four assertions.

  **The filter also applies to a caller-supplied environment**, not just the
  inherited one — a sanitizer covering only the inherited case is one an
  attacker routes around by passing the variable explicitly. Also
  mutation-verified. `rust-old` has **no test for any of this**; the roadmap
  already lists env sanitization as an untested area carried over from the Rust
  line.

- **M7 bite 3 — `sandbox/kavach_bridge`, the pure half.** Backend mapping,
  config construction, strength scoring, the output scan, and the trust-level
  policy table. 66 assertions; 15 of the oracle's 16 tests port.

  **`scan_output` is a lossy 4→2 collapse, and porting it "better" would be a
  silent divergence in both directions.** kavach's gate produces Pass, Warn,
  Quarantine and Block; Cyrius hands back all four, but Rust's `gate.apply`
  returns a `Result` — `Ok` for {Pass, Warn}, `Err` for {Block, Quarantine} —
  and the oracle maps those to Pass and Block. So the faithful port discards
  information it has: `{PASS,WARN}→PASS`, `{QUARANTINE,BLOCK}→BLOCK`.

  Worth recording *how* that got tested, because the first attempt did not.
  Asserting "the result is never WARN" is **vacuous** if no input produces WARN
  — and removing the collapse entirely still passed all 66. The fix was to find
  an input where the raw and collapsed verdicts genuinely differ: a **JWT** is
  scanned at `Severity.HIGH` (`lib/kavach.cyr:4184`), and the default policy
  quarantines at HIGH, so the raw verdict is QUARANTINE where the oracle answers
  Block. With that case in, removing the collapse fails by name.

  **Two oracle behaviours preserved that look like bugs.** `build_config` never
  reads `max_memory_bytes` — forwarding it would push a limit kavach never
  received in the Rust build. And `to_kavach_policy`'s seccomp branch is dead
  code, because `policy_basic()` already sets both fields; it is kept because
  the *starting point* is load-bearing — starting from `policy_new()` instead
  drops the +5 seccomp modifier and moves the wasm score 75 → 70 silently.
  Mutation-verified: that swap fails two exact-value assertions.

  **The oracle's `strength_ordering_matches_isolation` is more careful than its
  name.** It asserts only `native < wasm` and `native < process` — never
  `wasm < process`, because that is false: kavach scores WASM (65 base) above
  Process (50). A port that "completed" the ordering would fail. Asserted
  explicitly here, in the direction that actually holds.

  **All 16 oracle tests port**, after a same-day upstream round trip.
  `build_config_enables_externalization` was the holdout: Cyrius kavach's
  `SandboxConfig` had nine fields and no `externalization`, so the oracle's
  unconditional `.externalization(...)` had no counterpart and the test had
  nothing to assert against. agnosai filed it; **kavach 3.11.2** added
  `config_externalization(c, p)` (struct 72 → 80 bytes, defaulting to 0 so no
  existing consumer's behaviour changes), and `build_config` now attaches the
  policy exactly as the oracle does — unconditionally, at every isolation level
  including Noop. Mutation-verified.

  kavach carries the policy; it does not apply it. `scan_output` still calls
  `gate_apply` itself. What the field buys is that two sandboxes built for
  different crew trust levels are distinguishable from their configs, which is
  what `policy_for_trust` exists to produce.

- **M7 bite 2 — `sandbox/oci`, the pure half.** Config, image-reference
  validation, and the `docker run` argv. 75 assertions; seven of the oracle's
  eight tests port directly (the eighth needs a real spawn).

  **The argv is built as a VALUE, and that is the point.** The oracle assembles
  its command line inside `execute`, so `--read-only`, `--tmpfs /tmp` and
  `--network=none` — the entire isolation posture — have no test there. Here a
  test reads the vec, and every flag, its argument, and its *order* is pinned:
  `--memory` must be followed by its limit, the image must be last, and
  entrypoint arguments must come after it or the runtime would consume them.
  Mutation-verified — deleting `--network=none` fails a named assertion.

  **`validate_image_ref` is what stands between a caller string and argument
  injection**, so it is tested character-class by character-class rather than
  through the oracle's three examples: each shell metacharacter separately,
  embedded whitespace and newlines, and the leading-`-` flag-injection case
  (`--privileged`) versus a hyphen anywhere else, which is legitimate and must
  still be accepted. Mutation-verified.

  **`is_alphanumeric` is Unicode here too — no divergence.** `"unicode"` joins
  the stdlib deps, and the validator walks **codepoints** via `_uc_decode_utf8`,
  classifying with `unicode_category`: General Category `L*` union `N*`, which
  is what Rust's `is_alphabetic() || is_numeric()` resolves to.

  An ASCII-only validator was written first and was wrong to prefer. The
  argument for it — that a Cyrillic homograph of a registry host is the
  confusion an image validator should prevent — does not survive contact with
  what the string is used for: the reference goes into an **argv element**,
  never a shell string, so the validator's job is flag injection (the leading
  `-`) and shell-significant characters. A homograph is neither; it is a name no
  registry resolves, and docker fails on it either way. Rejecting it early
  bought nothing and cost oracle parity, which is the bar.

  Two things the oracle cannot express and this does: **malformed UTF-8**
  decodes to U+FFFD (category `So`) and is refused, where a Rust `&str` cannot
  hold invalid UTF-8 at all; and a non-ASCII **symbol** (emoji, non-breaking
  space) is still refused, which is the assertion that fails if a port fakes
  Unicode-awareness by accepting "any byte ≥ 0x80". One corner where they
  genuinely differ: Rust's `is_alphabetic` is the Unicode *Alphabetic property*
  (`L*` + `Nl` + `Other_Alphabetic`), and `unicode_category` exposes General
  Category, so a bare combining mark is refused here and accepted by Rust — it
  cannot appear in a resolvable reference and no test in either tree reaches it.

  Also preserved: `#` **is** in the oracle's allowed set (`oci.rs:80`) even
  though the doc comment three lines above omits it; the code is the oracle, and
  it is noted so nobody "fixes" it out.

- **`lib/unicode/` is vendored by hand, and the snapshot gate now recurses.**
  `cyrius lib sync` copies only the top level, so declaring `"unicode"` does not
  bring `lib/unicode/`'s seven files — they were copied manually. Nothing
  upstream keeps them current, so `scripts/check-clean.sh`'s snapshot check was
  made recursive; it is the only thing that notices if they go stale.
  Mutation-verified: editing one byte of `lib/unicode/categories.cyr` now fails
  the gate by name.

- **M7 bite 1 — `sandbox/policy` + the group hub.** Isolation levels, the five
  named policies, `effective_isolation`, the JSON wire, and the shared
  env-sanitization list. All 11 oracle tests port directly; 90 assertions.

  **Two things the oracle's tests do not cover, both pinned here.** First, the
  wire: `IsolationLevel` derives `Serialize` with no serde attributes, so serde
  emits the **variant name** — `"Oci"`, not `3`. The oracle's lone round-trip
  test cannot see the difference, because a port that emitted the discriminant
  would round-trip against itself perfectly; all four spellings are asserted in
  both directions. Second, `effective_isolation` when a tool needs **both**
  filesystem and network — not an oracle case, and the one where checking the
  flags in the other order silently answers `Process` for the tool asking for
  the most access. Both mutation-verified.

  **The port plan's closed blocker #5 — "rename to `AgnSandboxPolicy`" — does
  not apply.** That existed because Rust had two `SandboxPolicy` types in one
  namespace. The Cyrius hazard is sharper and differently shaped: kavach exports
  its constructors **unprefixed** (`policy_new`, `policy_basic`, `policy_strict`
  at `lib/kavach.cyr:2972-3007`), so a module reaching for the short spelling
  would bind to kavach's silently. The `agnosai_*` prefix rule already prevents
  it, and `check-symbols.sh` enforces the rule.

  `SANITIZED_ENV_VARS` has **no oracle test at all** — an untested security
  control the roadmap already lists under *Carried over from the Rust line*. It
  is tested now, before either subprocess backend lands: exact-match only, no
  prefix matching, case-sensitive (matching the loader's own matching, rather
  than implying a protection the loader does not provide), and the macOS pair
  stripped on Linux too.

- **SSE streaming — M6 bite 15c, and the milestone is complete.**
  `GET /api/v1/crews/{id}/stream` streams crew events for real:
  `text/event-stream`, chunked, `Cache-Control: no-cache`, `event:`/`data:`
  frames carrying whole serialised `CrewEvent`s, a 15-second `:ping`
  keep-alive, and the oracle's three termination paths. The deliberate 501 is
  gone from the transport.

  **The plan drafted for this bite was refuted 3/3 on adversarial review, and
  the headline finding is worth keeping.** The tempting justification for
  shipping no concurrency cap — *the oracle's `ConcurrencyLimitLayer(100)`
  starves at 100 streams too* — is **false**. tower holds its permit in
  `ResponseFuture`, which drops when the handler returns a *response*, not when
  the *body* finishes; `crew_stream` has **zero `.await` before `return`**, so
  the permit is held for microseconds and hyper streams the body outside the
  semaphore. The oracle therefore serves effectively **unbounded** concurrent
  streams while the port pins one of 100 pool workers per stream. That is a real
  operational divergence and is now
  [ADR 014](docs/adr/014-sse-stream-holds-a-pooled-worker.md).

  The other findings, each addressed rather than argued away:

  - **`Closed` is a flag on our own subscription struct**, set beside
    `chan_close` at both sites that close one. Bus membership is not a sound
    proxy — `agnosai_event_bus_sender` is get-or-create, so a removed crew id
    can reappear and *un-fire* the terminator, leaving a stream that never ends
    and never releases its worker. Reading `chan`'s own closed byte was also
    rejected: `src/chan_lossy.cyr` already recorded the house position against
    coupling to that layout.
  - **`Lagged` terminates *before* draining.** tokio surfaces it at the gap, and
    the oracle `break`s; here the counter is raised at send time with up to 256
    events still buffered, so the lag check runs **first in each iteration**.
    Draining first would emit 256 frames the oracle discards.
  - **Ids are canonicalised.** `agnosai_uuid_parse` accepts uppercase hex while
    crew ids are minted lowercase, and the oracle's `Path<Uuid>` +
    `to_string()` is case-insensitive by construction. Without
    `agnosai_uuid_canonical` an uppercase id passed the route gate, missed the
    bus, and reported "crew not found" for a running crew.
  - **All three oracle `warn!`s and all three serialisation fallbacks** are
    ported, not two — and they are reachable here, because
    `bayan_json_v_build_a` returns 0 on arena exhaustion where
    `serde_json::to_string` effectively cannot fail.

  23 assertions across four cases, including a live stream driven by a feeder
  thread. Verified end to end against the running binary with `curl -N`.


- **Two gates that close the version-skew class**, both mutation-verified rather
  than asserted.

  `scripts/check-clean.sh` now runs **`cyrius deps --verify`** (lockfile against
  the working `lib/`). Restoring the stale hash makes it print
  `FAIL: lib/kavach.cyr (hash mismatch)` and exit **1**; the clean tree reports
  `105 verified, 0 failed`. One trap worth recording: the exit code must be read
  directly, because piping through `tail` reports *`tail`'s* status and a gate
  written that way reads a real failure as a pass.

  CI gains a **Lockfile is honest** step — `git diff --exit-code -- cyrius.lock`
  immediately after `cyrius deps`. Since a runner resolves tags only, a committed
  lock that disagrees with a clean tag-only resolution *is* the local/CI
  divergence, caught at the one moment it is visible.

  Also recorded, and deliberately not acted on: `cyrius lib sync --full` copies
  **99 `.cyr` files** and does not recurse, so the 6.5.5 snapshot's `lib/unicode/`
  directory never lands. Its only consumer is `niyama.cyr`'s `\p{NAME}` regex
  support, which agnosai does not declare, and the failure mode if it ever did
  would be a loud undefined `unicode_category` at build rather than silent
  misbehaviour.

- **`/metrics` reports real numbers — the ADR 011 producer is wired.**
  [ADR 011](docs/adr/011-metrics-endpoint-serves-agnosai-metrics.md) gave the
  endpoint agnosai's own registry and explicitly staged the recording side as a
  separate `crew_runner` bite; until now it served a well-formed exposition of
  **zeros**. `agnosai_crew_runner_run` records the crew gauge on entry and on
  every exit, and a new `_agnosai_crew_record_metrics` folds each run's results
  into `tasks_completed`, `tasks_failed`, `tokens_total` and `cost_micro_usd`.

  Recorded in **one pass after the workers join**, not at the four
  result-construction sites. Every path lands in `results` — success, tool error,
  LLM error, and the cancellation stub — so one loop cannot miss an arm the way
  four call sites drift, and the token and cost figures are already on each
  result's metadata, so nothing has to be threaded down. The recorders were
  already atomic (`atomic_fetch_add`, plus a CAS loop for the saturating active
  gauge), so worker-thread recording would have been safe; it is simply not needed.

  Note the oracle defines `record_crew_started` / `record_crew_completed` and
  **never calls them** — its `/metrics` reports zero crews too. Wiring them is
  not a new divergence: ADR 011 names "crews created and active" in its decision
  and its Consequences section stages exactly this bite.

- **server_serve (M6 bite 15b) — the sandhi adapter, and the port's first
  `sandhi_server_*` call site.** 80 assertions. Four things had to land
  together: blocker #3's per-request arena, the allocation measurement, the a2a
  callback dispatch bite 12 left open, and the `rate_limit` mounting decision.

  **The handler is tested end to end over a pipe.** `agnosai_serve_handler`
  writes to a raw fd, so a pipe stands in for the socket and the whole adapter
  runs for real — sandhi's request parse, the router, the handler, the response
  encoder — everything but the accept loop. That test is what caught the
  cstring defects below; none of them is visible from reading the module, and
  all three would have shipped as a server that answered nothing correctly.

  **`rate_limit` is NOT mounted by default**, matching the oracle, which never
  installs the middleware. `agnosai_serve_with_rate_limit` is the opt-in path.
  Mounting it silently would be a wire change: clients fine today would start
  seeing 429s at a threshold agnosai chose, not one the oracle documents.

  **The arena covers sandhi's half only.** Every sandhi call threads `_a` and the
  worker rewinds between requests, so the transport half is flat. But
  `bayan_json_v_parse_buf` / `_build` thread no allocator and bayan ships no
  `_a` variants (verified: zero matches for
  `^fn bayan_json_v_(parse_buf_a|build_a)\(` in `lib/bayan.cyr`), so every
  handler that reads a body or builds a JSON response still grows the no-free
  global bump. The honest statement is *the transport is flat; the handlers are
  not* — measured, not asserted, and the residual is a bayan filing rather than
  a claim.

- **server_router (M6 bite 15a) — the route table, path matching, and the auth
  boundary.** 90 assertions against an oracle that has **no `#[cfg(test)]`
  module at all**, so every routing rule is covered for the first time.

  **Split from the sandhi adapter deliberately.** This is the router's
  *decision* — which handler runs, whether auth gates it, what comes back — and
  it touches no sandhi, so it is fully testable now. Bite 15b is the transport:
  `sandhi_server_run_pooled`, the per-worker arena (blocker #3), the
  `alloc_used()`-flat regression test, and the a2a callback dispatch.

  **The auth boundary is the security-critical part, and it is inverted on
  purpose.** `mod.rs:79-88` nests `/api/v1` plus the bare `/mcp` inside the auth
  middleware and merges that into a public router. The port writes the predicate
  as an allow-list of the **public** routes rather than of the protected ones,
  so a route added to the table without thought defaults to **protected** — the
  safe direction. Mutation-verified: flipping that default fails **20**
  assertions, and quietly dropping `/mcp` from the protected set fails 2.

  Three behaviours worth stating rather than discovering:
  - **`/health`, `/ready` and `/metrics` are always public**, even with auth
    enabled. `/metrics` unauthenticated exposes crew counts, task counts and
    inference cost.
  - **`/mcp` is protected but is not under `/api/v1`** — the entry most easily
    missed when transcribing the router.
  - **Routing precedes middleware**, so an unauthenticated request to a
    nonexistent path is **404, not 401** — an information disclosure the oracle
    accepts. And `DefaultBodyLimit` is the *outermost* layer, so an oversized
    body is **413 before routing**, hence before auth, even on an unknown path.

  A known path under an unregistered method is **405, not 404**. Path parameters
  match exactly one non-empty segment and never span `/`, so `/crews//cancel`
  and `/crews/a/b/cancel` both miss; the captured value reaches the handler,
  which is why a malformed crew id is the handler's 422 rather than the router's
  404.

  **`/api/v1/crews/{id}/stream` answers 501, not 404** — the route exists and
  only its handler is missing (SSE is bite 15c); a 404 would tell a client the
  endpoint is not part of the API, which is a different and wrong statement. It
  is still auth-gated.

  **`ConcurrencyLimitLayer` has no sandhi analogue** — tower queues above the
  limit, while sandhi's pooled server bounds work in flight by worker count and
  does not queue. `AGNOSAI_MAX_CONCURRENT_REQUESTS` is carried as a constant so
  bite 15b sizes its pool against the oracle's intent rather than inventing a
  number, and so the divergence is stated rather than dropped.

- **server_hot_config + server_rate_limit (M6 bite 14) — the last two pure
  files of the transport tier.** 28 and 41 assertions against the oracle's 5
  and 7. Both are still `fn(inputs) -> decision`; neither needs sandhi.

  **`ConfigHolder<T>` is monomorphised, and `watch` becomes an atomic pointer
  swap.** The oracle's holder is generic and its tests instantiate it at `u32` /
  `String` / `u64` purely to prove the container round-trips; Cyrius has no
  generics and the tree has exactly one instantiation, so those port as
  `RuntimeConfig` round trips. tokio's `watch` is single-writer/many-reader with
  no reader contention — under `run_pooled` every worker reads this per request,
  so the equivalent is an **atomic store**, not a mutex. A whole config is
  published by one 8-byte pointer store, so a reader sees the old one or the new
  one, never a torn mix; **8 threads × 500 reads against 400 concurrent
  publishes** pin it. The previous config is deliberately not freed — a reader
  may still hold it, which is what the `Arc` was for, and reloads are
  operator-driven so the bound is reload count, not traffic.

  **`RuntimeConfig` has no `#[serde(default)]` on any field**, so `Default`
  applies only when nothing is deserialized at all — a **partial object is a
  failure, not a merge with the defaults**. That is the tempting misreading and
  it would silently accept configs the oracle rejects.

  **`rate` is integer thousandths**, not an f64: majra's `ratelimit_new` takes
  `rate_x1000`, so the oracle's `10.0` is `10000`, and the bucket arithmetic
  stays exact — the same treatment money and throughput already get.

  Two oracle behaviours reproduced with the reasoning written down rather than
  quietly improved: **`"unknown"` is a single shared bucket** for every client
  presenting neither `X-Forwarded-For` nor `X-Real-IP`, so exposed directly one
  noisy client rate-limits everyone (the oracle's own comment says
  "single-bucket for all clients"); and an **empty XFF keys the empty-string
  bucket rather than falling through** to `X-Real-IP`, because
  `s.split(',').next()` always yields at least `""` and the oracle returns it
  trimmed without an emptiness check.

  **A test caught the gate ordering, not the code.** `to_str()`'s
  `is_visible_ascii` admits 0x20-0x7E plus tab, so a header carrying CRLF fails
  it and is treated as **absent** — the trim never runs. My first test expected
  CRLF to be trimmed; the implementation was right and the expectation wrong.
  Now pinned as the interaction: tab is trimmed, CR/LF make the header absent
  and fall through. Trimming first would have silently accepted a
  header-injection shape.

  `rate_limit` is still **not mounted** — `server/mod.rs:47-99` never installs
  it, and the port reproduces that. But `auth_jwt_verify_ok`'s 3.3 ms means a
  core sustains ~300 JWT verifies/second, and the `alg` check sits after the
  signature so a malformed token costs the same — making an unauthenticated
  flood a real amplification vector. Mounting it is a decision for the router
  bite, recorded in state.md.

- **server_routes_mcp (M6 bite 13) — Model Context Protocol over JSON-RPC 2.0.
  The routes tier is complete.** 80 assertions against the oracle's 5.

  **The envelope is built here, not delegated to bote.** The oracle uses
  `bote::protocol::{JsonRpcRequest, JsonRpcResponse}` and
  `bote::registry::{ToolDef, ToolSchema}` **as types only** — it says so at
  `mcp.rs:3-5` and never calls bote's dispatcher. The Cyrius side makes that
  sharper: `lib/bote-core.cyr`'s `dispatcher_dispatch` (`:1713`) hardcodes
  `"serverInfo":{"name":"bote"` in `_build_initialize_result`, where the oracle
  emits `"agnosai"`. Delegating would have changed the wire.

  **The shapes are transcribed from the Rust bote 0.91.0 the oracle pins**, not
  from the Cyrius bundle, because the oracle's wire format is whatever serde
  emits there. That surfaced three `skip_serializing_if` behaviours **no oracle
  test checks**, now pinned: a success **omits** `error` (not null), an error
  **omits** `result`, and the error object **omits** `data`. Likewise
  `ToolSchema`'s `#[serde(rename = "type")]` — a literal transcription of the
  Rust field name would have emitted `schema_type`, which no MCP client parses.

  **The asymmetry most likely to be flattened**, and the mutation that proves it
  is tested: a **missing tool name** is a protocol-level JSON-RPC `-32602`,
  while an **unknown tool** is a JSON-RPC *success* carrying `isError: true`.
  Tool-level fault versus protocol-level fault. Similarly a non-object
  `arguments` yields an empty parameter map rather than an error, so the tool
  runs and reports the fault itself.

  Also pinned: a **string** tool result is emitted bare, not re-quoted as JSON
  (the oracle's own `tools_call_executes_echo_tool` asserts `"hello"`, not
  `"\"hello\""`); every arm is HTTP 200, since JSON-RPC carries faults in the
  body; and `id` is echoed on every arm including errors, defaulting to JSON
  null when absent.

  **agnosai still calls nothing from bote.** This module was its only intended
  consumer and it consumes none of it — worth weighing against the ~93 KB and
  233 fns `[deps.bote]` adds to every build.

- **server_routes_a2a (M6 bite 12) — Agent-to-Agent task delegation.**
  72 assertions against the oracle's 5.

  **The callback's SSRF decision is ported; its dispatch is not, deliberately.**
  The oracle ends `receive` by `tokio::spawn`-ing a fire-and-forget POST to
  `callback_url` under a 30-second timeout, explicitly ignoring the result. This
  module ports the **decision** — `agnosai_route_a2a_callback_allowed`, the SSRF
  gate, which is the security-relevant half and the half all five oracle tests
  cover — and leaves the POST to the transport tier, for two reasons that point
  the same way: outbound HTTP goes through `agnosai_guarded_fetch`, which needs
  the per-request arena the router bite introduces, and spawning a thread per
  callback is a resource decision belonging to whoever owns the thread pool.
  The handler is complete without it — the oracle's own response does not depend
  on the callback either — and a test pins that an **SSRF-rejected URL does not
  fail the request**, matching the oracle's warn-and-fall-through.

  **The failure arms keep the `A2AResponse` shape**, not the routes tier's
  `{"error": ..}`: the oracle types this handler `(StatusCode, Json<A2AResponse>)`,
  so a 400 still carries `task_id`, `status: "failed"`, a null `result` and the
  message in `error`. Pinned, because returning the tier's generic error shape
  is the obvious thing to do and it changes the wire.

  **The metadata cap is measured on the re-serialized value**, matching
  `serde_json::to_vec(&req.metadata)` — so whitespace in the incoming body does
  not count against the 64 KiB limit. `metadata` is the one field with no type
  constraint (`serde_json::Value`), and object, array, scalar and null are all
  tested as valid.

  The oracle has **no test for `receive` at all**, so the length caps, the
  metadata cap, the crew-name construction (`a2a-{domain|general}-{task_id}`),
  the response shape and every rejection arm are new. Its five tests are all
  `is_safe_callback_url`, a one-line delegate to the SSRF module already covered
  by `server_ssrf.tcyr`'s 81 assertions; ported anyway, so a refactor that
  stopped delegating is caught at this level too.

- **server_routes_crews, handlers (M6 bite 11b) — `crews.rs` is now ported
  whole.** `create_crew`, `get_crew`, `cancel_crew`, and the oracle's 4
  `#[tokio::test]`s. **124 assertions across both bites**, against the oracle's
  10.

  **One enum, two spellings, both deliberate and both now pinned.**
  `create_crew` renders statuses with `serde_json::to_value`
  (`crews.rs:207-222`), so this file emits the **snake_case** wire form
  (`completed`), while `dashboard.rs` formats the same enum with `{:?}` and
  emits `Completed`. A test asserts each spelling *and* the absence of the
  other, so collapsing them into one renderer fails loudly.

  **`process` maps through a catch-all, which is why the reader does not
  validate it**: anything that is not exactly `hierarchical` / `dag` /
  `parallel` — absent, empty, or misspelled — runs Sequential. `hierarchical`
  gets a fresh UUID as its manager, matching the oracle's placeholder
  `Uuid::new_v4()` that names no real agent; carried across rather than
  improved, since inventing a real manager would change which agent runs the
  crew. Concurrency follows `unwrap_or(4).clamp(1, 64)`, with all four boundary
  cases tested.

  **Index-based dependencies on the wire become UUID-based in the spec**, in the
  two-pass shape the oracle needs "to avoid simultaneous borrow" and this one
  needs because a task's id does not exist until it is built.

  **A bug caught while reading, not by a test**: `agnosai_orchestrator_cancel_crew`
  follows the port's error convention where **0 is success** and non-zero is the
  error object — the first draft checked `!= 1` and would have answered 404 for
  every successful cancellation. `cancel_crew` has **no oracle test at all**, so
  nothing upstream would have caught it either; it now has five assertions.

  Also new relative to the oracle: `get_crew`'s found path, and that a malformed
  id is **422 rather than 404** on both id-taking routes — axum's `Path<Uuid>`
  extractor rejects before the handler, the same extractor-owns-the-status
  situation `agents.rs` and `approval.rs` have.

  One honest note: the `skip_serializing_if` profile omission is **not reachable
  through this handler**, because `run_crew` always attaches a profile
  (`orch_crew_runner.cyr:1066`). No test isolates it and a mutation rendering
  unconditionally passes the suite; kept for the oracle's shape, and said so.

- **server_routes_crews, validation half (M6 bite 11a) — the request types,
  `validate_crew_request` and the cycle detector.** 73 assertions.

  **Split because the oracle's own tests split.** `crews.rs` is 528 lines, the
  largest remaining handler, and CLAUDE.md's sizing rule says break it up. The
  natural seam is the oracle's: **6 of its 10 tests are plain `#[test]`, not
  `#[tokio::test]`, and all six cover only this half** — so it lands complete
  and verified before any handler exists. `create_crew` / `get_crew` /
  `cancel_crew` are bite 11b.

  **`priority` is validated; `process` deliberately is not.** Both are
  `Option<String>`-ish in shape, and the correct treatments are opposite.
  `TaskPriority` is a typed enum, so serde rejects `"priority":"urgent"` — and
  `agnosai_task_priority_from_wire` would have silently answered Normal, the
  same coercion trap `approval.rs` hit. `process` really is `Option<String>` in
  the oracle, matched *after* deserialization with a `_ => Sequential`
  catch-all (`crews.rs:161-171`), so an unrecognised name is accepted and runs
  sequentially; rejecting it in the reader would have been the divergence.

  **`agents` and `tasks` are required, not defaulted** — neither carries
  `#[serde(default)]` — so an absent one is a 422 with no message while a
  present-but-empty one reaches validation and produces a 400 with a specific
  one. Both paths pinned, because collapsing them is the obvious simplification
  and it changes the wire.

  Error-message text and check *order* are both reproduced: agents are tested
  before tasks, so a request missing both reports the agent message, and every
  dependency index is range- and self-checked before the cycle sweep.

  One honest note in the module header: the cycle detector's two marks carry
  different weight. `== 1` (in-progress) is the correctness property — deleting
  it recurses forever and the suite catches the stack overflow. `== 2` (done) is
  **purely an optimisation**: deleting it changes no answer, so no assertion
  isolates it and a mutation removing it passes the whole suite. It is kept
  because without it the walk is exponential on a graph where each task depends
  on the previous two — 45 such tasks is ~10⁹ visits, inside the oracle's
  1000-task cap and reachable from an unauthenticated body.

- **server_routes_dashboard (M6 bite 10) — crew history and agent performance.**
  39 assertions against the oracle's 2 — **and both of the oracle's are the
  empty case**, so it never renders a single crew or agent. Every populated path
  is covered here for the first time.

  Adds `agnosai_orchestrator_crew_ids` (`src/orchestrator/orchestrator.cyr`), the
  accessor this route needed. **The oracle's read guard does not port**: both
  handlers hold `state.orchestrator.state().read().await` across the whole walk,
  which here would stall every worker's crew lookups for a dashboard render —
  the same trade `tools_registry.list` and the definitions snapshot already
  refused. The ids come out under the lock and the walk runs unlocked, so a crew
  removed mid-walk is skipped rather than rendered. Documented rather than
  glossed: the port is not claiming the oracle's snapshot consistency.

  Two oracle behaviours reproduced rather than tidied, each pinned:
  `format!("{:?}", c.status)` emits the **capitalized** `Debug` variant name —
  `Completed`, not the `completed` every other endpoint emits via
  `agnosai_crew_status_to_wire` — so it gets its own renderer rather than
  sharing one; and **`tool_count` is hardcoded 0** in the oracle, not derived
  from anything, so a dashboard consumer reading a real count would be reading
  something the Rust never sent.

  **The tests caught a real defect before it shipped.** `agent_performance` read
  result metadata with `bayan_json_v_obj_get`, but
  `agnosai_task_result_metadata` is a stdlib Str-keyed `map` whose *values* are
  JSON — mirroring the oracle's `HashMap<String, serde_json::Value>` — so the
  lookup silently found nothing and the endpoint would have returned `[]` for
  every request. Now `map_get`, with the trap written into the accessor's header.

- **server_routes_approval (M6 bite 9) — the human-in-the-loop endpoints.**
  41 assertions against the oracle's 2. The first request type carrying
  `#[serde(deny_unknown_fields)]`, so the first real consumer of
  `agnosai_route_no_unknown_fields`.

  **`ApprovalDecision` is validated, never coerced — and that distinction is the
  whole bite.** `agnosai_approval_decision_from_wire` maps anything unrecognised
  to **Rejected**, which is the right fail-closed answer for its own caller (a
  crew callback, where a garbled decision must not become an approval). Reusing
  it here would have turned `{"decision":"aproved"}` — a typo — into a rejection
  the operator never asked for, answered 200, and cleared the pending approval.
  So the route validates the spelling explicitly and 422s otherwise; a test pins
  that the task is *still pending* afterwards.

  **200 with `delivered: false` for an unknown task**, not 404 — the tempting
  shape and the wrong one; the oracle's own test asserts it. And the message
  carries the capitalized `Debug` variant name (`Decision Approved delivered`)
  even though the request spelled it snake_case, the same `{:?}`-vs-serde
  asymmetry the dashboard has.

  The oracle only ever exercises the undelivered path and an empty pending list,
  so the delivered path, the populated listing, and the whole 400/422 rejection
  surface — malformed JSON, unknown field, missing fields, invalid UUID,
  wrong-typed fields — are new.

- **server_routes_agents (M6 bite 8) — agent-definition listing and creation.**
  The first real consumer of `server_state`'s `definitions` map. 38 assertions
  against the oracle's 5.

  **This handler owns its own deserialization, unlike the rest of the tier.**
  Three of the oracle's five tests assert the status of a body that never
  reaches its handler — axum's `Json<AgentDefinition>` extractor rejects it
  first — so `agnosai_route_create_definition` takes the **raw body** and owns
  the whole contract: **400** for malformed JSON (the oracle's test accepts 400
  or 422; 400 is the more precise for a syntax error), **422** for valid JSON
  that is not an `AgentDefinition`, **500** for the oracle's unreachable
  serialization arm, **201** with the definition echoed back. Splitting it the
  other way would have left the port's most commonly hit failure paths
  untestable until the transport bite. `agnosai_agent_from_value` already gates
  on exactly serde's required set, so the 422 boundary agrees with the oracle by
  construction rather than by a second list kept in sync.

  **The oracle never tests the success path** — it has no test that posts a
  valid definition. That half is covered here for the first time: the 201 body,
  storage keyed by `agent_key` (not by `name`), replace-on-duplicate, optional
  fields surviving the round trip, and create-then-list agreement.

  One honest note in the module header: the `filter_map`-equivalent skip in
  `list_definitions` is **unreachable**, because `agnosai_agent_to_value` has no
  failure path where serde's `to_value` does. It is kept for the oracle's shape
  and because it would matter if that ever changes, but it is documented as
  unreachable rather than implied to fire — a mutation deleting it passes the
  whole suite, which is exactly why the comment says so.

- **server_routes_tools (M6 bite 7) — tool listing, runtime unregistration, and
  preset listing.** 34 assertions against the oracle's 4. `tools.rs` and
  `definitions.rs` land together because the latter is four lines of behaviour.

  **The oracle has no test for `remove_tool`**, so its 204/404 branch — the
  whole substance of that handler — is covered here for the first time,
  including remove-twice, unknown-name, and that a 404 leaves the registry
  untouched.

  **`GET /api/v1/presets` returning `[]` is exact parity, not a stub.** The
  oracle's body is `#[cfg(feature = "definitions")]`-gated and agnosai's
  `default = []` leaves that feature off, so the default cargo build — which is
  what the port targets — returns an empty array, and the oracle's own test
  asserts precisely that. The populated arm arrives with M10.

  Empty collections answer `[]` and never `null`, pinned explicitly: the
  oracle's `Vec` serializes to an empty array, and a handler returning the
  registry's empty vec unconverted would produce `null` and break clients.

- **server_routes + server_routes_health (M6 bite 6) — the routes tier opens.**
  The response vocabulary every handler will speak, the shared
  `deny_unknown_fields` guard, and `health.rs`'s three probes. 42 assertions
  against the oracle's 3.

  **Handlers are `fn(state, inputs) -> response`.** The oracle's are
  `async fn(State<..>, Json<T>) -> impl IntoResponse` and its tests drive them
  through `app.oneshot(..)`, which is why 43 of M6's remaining oracle tests are
  `#[tokio::test]` — almost none because the *logic* is async. All three of
  `health.rs`'s ported as ordinary assertions, as `server_auth`'s ten did.

  **`deny_unknown_fields` is written once, here, not four times later.** serde
  enforces it during deserialization on `TaskRequest` (`crews.rs:14`),
  `CrewRunRequest` (`:32`), `A2ARequest` (`a2a.rs:34`) and `ApprovalSubmission`
  (`approval.rs:12`); bayan has no equivalent, so without it the port would
  **accept request bodies the oracle rejects** — a fail-open divergence that no
  oracle test guards. Pinned by 10 assertions including the empty-allow-list case
  a loop bug would silently pass; mutation-verified.

  **`/metrics` serves agnosai's registry, not hoosh's —
  [ADR 011](docs/adr/011-metrics-endpoint-serves-agnosai-metrics.md).** The
  oracle returns `crate::llm::llm_metrics::gather()`, and `llm/mod.rs:26` is
  `pub use hoosh::metrics` — so in the Rust build the endpoint exposes the
  **hoosh crate's in-process registry**. Under the HTTP seam
  ([ADR 003](docs/adr/003-llm-native-http.md)) there is no in-process hoosh and
  nothing to read, so it serves `server/prometheus.rs`'s port instead. That also
  gives `AgnosMetrics` its first consumer: it has **zero references** anywhere in
  the Rust tree — dead code the oracle carried but never wired. Names are all
  `agnosai_`-prefixed, so scraping agnosai and hoosh into one Prometheus needs no
  relabeling. **Producer side is a separate bite**: the oracle records at
  `crew_runner.rs:810`/`:864` into hoosh's registry, so until the equivalent
  calls land in `src/orchestrator/crew_runner.cyr`, `/metrics` renders a well-formed
  exposition of zeros — which the oracle's own test still accepts, since it
  inspects no metric names.

  `AGNOSAI_VERSION` is the one place the version number is duplicated — Cyrius
  has no `env!("CARGO_PKG_VERSION")` equivalent — and must track the root
  `VERSION` file; CLAUDE.md's work-loop version check covers it.

- **server_state (M6 bite 5) — `AppState`, the keystone for the routes tier.**
  Small file, load-bearing position: **13 of the 18 route entries registered in
  `server/mod.rs:48-99` take `State<SharedState>`**, so no `routes/*` handler can
  be written until it exists. 31 assertions against an oracle that has **no test
  module at all** — `state.rs` is a struct plus a type alias, and every route test
  builds one as a fixture.

  **`Arc<T>` is a raw pointer.** The oracle wraps `tools` and `audit` in `Arc` and
  the struct in `Arc<AppState>`; the port plan already settled that 49 of the
  tree's 52 `Arc<` are process-lifetime shared-immutable state. `AppState` is
  built once at startup and outlives every request, so there is nothing to
  refcount.

  **`http_client` is dropped, deliberately.** `reqwest::Client` is a
  connection-pool handle; agnosai's outbound HTTP goes through
  `agnosai_guarded_fetch(arena, ...)`, which takes a per-request arena and no
  client object. The loss is thinner than it looks: of the twelve `http_client`
  references in `rust-old/src/server/`, **eleven are test fixtures** and the only
  real consumer is `a2a.rs:153`. If pooling is ever wanted it belongs in
  `guarded_fetch`, not in the state struct.

  **`definitions` is a map behind a mutex, and the mutex is the whole bite.** The
  oracle uses `DashMap`, which is lock-free and concurrent; under
  `sandhi_server_run_pooled` every worker is a real OS thread, so an unguarded
  write during a concurrent read corrupts the table. `tools_registry` set the
  precedent and this follows it. That is the one property `DashMap` gave the
  oracle free and this port has to earn, so it is tested with **8 real threads
  inserting 200 disjoint keys each** and asserted for an exact final count —
  removing the lock fails it reproducibly, 3 runs out of 3.

  Two further behaviours are pinned because they are easy to regress silently:
  the key is **cloned** on insert (a caller's Str may borrow from a request
  buffer that does not outlive the handler — the test scribbles over the caller's
  bytes and requires the stored key to survive), and `agnosai_app_state_definitions`
  returns a **detached snapshot** taken under the lock rather than a live view.
  The oracle's `DashMap::iter()` holds per-shard guards while the caller walks it,
  which here would mean holding the mutex across serialization of every entry, on
  a path every worker shares.

- **server_auth (M6 bite 4) — the RS256 JWT half. `auth.rs` is now ported whole.**
  PEM → `(n, e)` decoding, strict base64url, field-based `alg` pinning, signature
  verification, and claim validation. 112 assertions in total across both bites,
  against the oracle's 10.

  **`pem_decode_pubkey` really was ~15 lines of local glue**, as the port plan
  said: `_pem_find` / `_pem_b64_decode` take the label as a parameter and
  `rsa_pubkey_from_der` already accepts SPKI, so only the `PUBLIC KEY` label pair
  was missing. Both it and `RSA PUBLIC KEY` are accepted, matching
  `DecodingKey::from_rsa_pem`.

  **Two maintainer decisions landed, both recorded:**
  - **`iss`/`aud` TIGHTENED** ([ADR 010](docs/adr/010-jwt-require-configured-iss-aud.md)).
    jsonwebtoken's `set_issuer`/`set_audience` set only the expected value and
    never add the claim to `required_spec_claims` (`validation.rs:143-145`), so
    the oracle accepts a token carrying **no `iss` at all** even with an issuer
    configured. Since `JwtConfig` holds one static key with no `kid` routing,
    `iss`/`aud` are the only cross-tenant separation the design has. Now required
    when configured — conditionally, mirroring the oracle's `if let Some` shape,
    so the unconfigured path is unchanged. Zero oracle assertions change.
  - **The 60-second `exp` leeway REPRODUCED** as a named constant citing
    `validation.rs:120`. It is invisible from `auth.rs` — the oracle never sets
    it, it comes from `Validation::new` — so a port written faithfully from the
    oracle *source* would have compared `exp < now` and silently shipped a
    stricter server.
  - Array-valued `aud` is **inherited** as a rejection (fail-closed), including
    the single-element `["agnosai"]` form that Auth0 and Cognito emit.
  - The fixture `exp` is `253402300799` (9999-12-31), replacing the oracle's
    `u64::MAX`, which is `2*i64::MAX+1` and does not fit Cyrius's signed i64.

  **`alg` is pinned as a parsed JSON field, never a substring.** sigil exposes
  only the raw RSA primitive, so there is no `Validation::new(Algorithm::RS256)`
  to inherit; without this the port would ship an `alg:none` bypass the oracle
  does not have, and the oracle's own tests never exercise `alg`, so test parity
  would not have caught it. A vector with `{"alg":"none","kid":"RS256-2024"}`
  pins that a substring scan would not do. Verified by mutation: deleting the
  check fails exactly the three assertions that isolate it.

  **`now` is injectable** (`agnosai_auth_check_at`), which is the only way to
  drive the leeway boundary against a frozen token vector — the suite checks
  exp+30, exp+60 (accepted) and exp+61, exp+90 (rejected).

  **The key is parsed once, not per request.** The oracle rebuilds
  `DecodingKey::from_rsa_pem` inside `validate_jwt` on every authenticated
  request (`auth.rs:120-123`). `agnosai_jwt_config_prepare` hoists it — both the
  performance fix and the fix for a real hazard, since sigil's `_pem_init` guards
  its table build with a plain non-atomic flag and `run_pooled` makes every
  worker a thread. Observable behaviour is unchanged: a PEM that does not parse
  still answers 500 on every request reaching the JWT path.

  **base64url is decoded locally rather than through `bayan_base64url_decode`.**
  bayan's allocates three times per call on the no-free global bump
  (`lib/bayan.cyr:132`, `:150`, `:180`) — a per-request leak against the arena
  discipline the rest of the server follows — and is lax where a token verifier
  must not be: it strips trailing `=` and silently drops the stray character of a
  segment whose length is 1 mod 4. The local decoder allocates nothing, rejects
  both, and rejects non-canonical trailing bits, matching the `base64` crate's
  default that the oracle inherits.

  Frozen vectors live in `tests/server_auth_vectors.cyr`, signed once with the
  oracle's own keypair and confirmed with `openssl dgst -verify` before being
  committed — **agnosai contains no RSA signing and no test needs a private key.**

- **`scripts/check-symbols.sh` + a CI gate.** Two rules the compiler cannot
  enforce: no name defined twice in `src/` across **all** definition kinds (fn,
  var, enum member), and every top-level symbol `agnosai_*`/`_`-prefixed. The
  second rule closes the entire ~180-name unprefixed export surface of
  `lib/bote-core.cyr` at once, with no denylist to maintain. Runs before Build.

### Changed

- **`lib/mabda.cyr` re-synced 4.0.7 → 4.0.8** to match the 6.5.4 pin; `cyrius.lock`
  updated. `lib/` now matches the pinned snapshot exactly — 0 of the stdlib files
  differ — and the build emits no shadow or drift warning. (The remaining
  not-in-snapshot entries are the git deps: ai-hwaccel, bote-core, kavach, libro,
  majra, tyche.)
- **`docs/development/state.md`: two internal contradictions corrected.** The gate
  table said `server` was **20 of 21 files** while the handoff section 400 lines
  later said **6 of 21**; 20 is right. And the git-dep list said sigil 3.12.1 /
  bote 3.1.4 against `cyrius.cyml`'s 3.12.2 / 3.2.1 — the same hand-transcription
  drift the file's own 2026-07-30 handoff note warns about ("regenerate the tables
  from command output rather than editing rows by hand").

- **`src/` now mirrors `rust-old/src/`.** 65 of 71 modules moved out of the flat
  scaffold layout into the oracle's directory shape: `src/server_routes_crews.cyr`
  → `src/server/routes/crews.cyr`, `src/orch_crew_runner.cyr` →
  `src/orchestrator/crew_runner.cyr`, and a group's hub → `mod.cyr`. Directories
  take the oracle's spelling, so `orch_*` lands under `orchestrator/`.

  Nothing forced the flat layout — Cyrius `include` is textual and takes a path,
  and the cyrius compiler's own tree uses `src/backend/x86/emit.cyr`. The scaffold
  default had encoded the oracle's directories into filename prefixes, and
  `CLAUDE.md` then recorded that as if it were a language constraint. It read
  as one for the whole port. The correctness bar here is "matches what Rust did",
  judged file-against-file; the tree now shows that correspondence instead of
  requiring a reviewer to parse prefixes to recover it.

  **Verified by a byte-identical binary** — `sha256` unchanged at
  `dd151e53…f342a8` before and after. `include` is textual, so the same files in
  the same order preprocess to the same source; a hash match is proof the include
  order survived, which is the only way this refactor could have silently broken
  anything (a wrong order can resolve to a *different but still-compiling* symbol
  under last-definition-wins). All other gates unchanged: 1386 top-level
  definitions across 71 files, 57 suites green, coverage 100% (873/873 fns,
  64/64 files). All 65 are `git mv` renames, so `git log --follow` still works.

  Six files needed a decision rather than the rule, each noted where it lands:
  `server_router.cyr` + `server_serve.cyr` split the oracle's single
  `server/mod.rs` (bites 15a/15b), so neither claims `mod.cyr`;
  `server_routes.cyr` → `routes/mod.cyr`; `tools_builtin_basic.cyr` merges the
  oracle's `builtin/echo.rs` + `builtin/json_transform.rs`; `core_json.cyr` and
  `orch_audit.cyr` are port-local with no `rust-old/` counterpart. Port-local
  support modules (`units`, `order`, `id`, `guarded_fetch`, `chan_lossy`) stay at
  `src/` root.

  **Anything walking `src/` must now recurse.** `scripts/check-symbols.sh` and
  `scripts/check-clean.sh` had seven `src/*.cyr` globs that would have matched
  **nothing** after the move and passed — failing open, the dangerous direction.
  Both now use `find src -name '*.cyr'` / `glob(recursive=True)`, confirmed by the
  symbol count reproducing exactly. `cyrius coverage` and `cyrius tests` already
  recursed (`cyrius/cbt/quality.cyr:83`, `dir_walk_with_prunes`).

  No symbol renames: Cyrius has one flat namespace regardless of directory, so
  every `agnosai_*` prefix is untouched.
- **Toolchain pin 6.5.3 → 6.5.4, and sigil 3.12.1 → 3.12.2 with it.** Unlike the
  6.5.3 bump, this one **moves real stdlib source** and needed
  `cyrius lib sync --full`: `cyrius deps` re-layers the git deps but does not
  refresh the stdlib, so `lib/` sat at the 6.5.3 snapshot until the sync ran.
  The sigil pin had to move in the same commit — `cyrius deps` copies each git
  dep's vendored bundle into `lib/` with last-write-wins, so leaving the pin at
  3.12.1 would have overwritten the fold's 3.12.2 back down.

  Three items land on agnosai:
  - **`vec_sort_by` / `vec_select_nth` shipped**, closing agnosai's own filing
    `2026-07-28-agnosai-no-nlogn-sort-in-stdlib`. No collision — `src/order.cyr`
    is entirely `agnosai_*`-prefixed. Measured head-to-head at 100k, the stdlib
    introsort is **3.85× faster** than our heapsort (20.0 ms vs 77.1 ms) and its
    quickselect 1.34× (4.22 ms vs 5.65 ms); migrating is recorded as an open
    decision in state.md rather than taken here.
  - **sandhi 1.9.7 → 1.9.8 changes a return contract** the transport tier will
    depend on: the five serve loops previously spun a core forever on a
    persistent accept error and never returned once listening, and now return 1
    on a dead listener or after 200 consecutive resource failures. Nothing here
    calls `sandhi_server_*` yet, so it is a note for the router bite.
  - **sigil 3.12.2 fixes a 144-byte-per-call `sha256_init` leak** that **never
    affected us**: `_agnosai_auth_secret_eq` uses the banked, allocator-free
    `sha256()` one-shot. Confirmed by measuring the shared-secret path at
    **32 bytes/request** before and after.

  The long-standing `lib/sakshi.cyr` 2.4.3 shadow warning also cleared — it is
  now 2.4.7, matching the bundle. 44/44 suites and 771/771 coverage green after.
- **Toolchain pin 6.5.2 → 6.5.3.** `lib/` is byte-identical between the two tags
  (`git diff 6.5.2 6.5.3 -- lib/` is empty), so the bump moves no stdlib source and
  needed no re-verification beyond a full rebuild. It is bugfix-only upstream —
  correct diagnostic line numbers after an `include`, and an `install.sh` fix — and
  it clears the `manifest-pin: 6.5.2 (drift — wrapper is 6.5.3)` banner the
  installed CLI printed on every invocation. 43/43 suites still green after.

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
    `src/llm/pricing.cyr` — 16 rows, per-provider fallbacks and a truncating cost expression copied
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
- **llm** (M3, Phase 2) — **complete**. `src/llm/mod.cyr` hub plus three submodules:
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
- **core** (M2, Phase 1) — **complete**. `src/core/mod.cyr` hub plus all six oracle submodules:
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
  submodules of `rust-old/src/learning/` ported, with `src/learning/mod.cyr` as the hub mirroring
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

- Final Rust release line before the Cyrius port. `bench-history.csv` is frozen at this point and
  moves to `rust-old/`; the Cyrius tree starts a fresh baseline. tokio-era numbers are **not**
  comparable across the port — see `docs/development/cyrius-port-plan.md`.

### Fixed

- **An unauthenticated SIGSEGV on `GET /api/v1/crews/{malformed}/stream`**,
  found by the test written for it. `agnosai_route_resolve` matches
  `/api/v1/crews/*/stream` on *shape* — it never parses the `*` — and every
  other crew route validates in its own handler. The stream path did not, so a
  non-UUID segment reached `agnosai_uuid_canonical`, which answered 0, and
  `str_clone(0)` faulted. Any unauthenticated client could crash the process
  with one request.

  Fixed at both layers: the transport now rejects with **422** before streaming
  (matching `agnosai_route_get_crew` and the oracle's `Path<Uuid>` extractor),
  and `agnosai_sse_crew_stream` no longer faults on an unparseable id even if a
  caller skips the gate. Mutation-verified — removing the gate turns the 422
  assertion red instead of crashing, which is the second layer doing its job.

- **Graceful shutdown — the last gap in bite 16, closed by an upstream fix
  agnosai filed itself.** `./build/agnosai` now drains on SIGINT and SIGTERM,
  logging `"received shutdown signal, draining"` then the oracle's
  `"server shut down gracefully"` — a line that until now had nothing true to
  report. [ADR 013](docs/adr/013-graceful-shutdown-via-signalfd-and-stop-flag.md)
  supersedes [012](docs/adr/012-no-graceful-shutdown-on-sandhi.md), which shipped
  earlier the same day recording that this was *impossible*.

  It was impossible, and the reason was never missing signal support — that was
  available all along. A `sandhi_server_run*` loop simply could not be made to
  return: it read no flag, its only exit was a fatal accept, and its listen fd
  was loop-local and never published. ADR 012 named the one upstream change that
  would reverse it; agnosai filed it, **sandhi 1.9.9** implemented it, and
  **cyrius 6.5.6** vendors it.

  **Three orderings are load-bearing, and each is a real failure if inverted.**
  (1) `sys_sigprocmask` sets the *calling thread's* mask and new threads inherit
  it, so signals are blocked **before** `run_pooled` spawns workers — installed
  after, SIGTERM would kill a worker mid-request instead of draining. (2) The
  block must precede the `signalfd`, or the signal is delivered conventionally
  and kills the process. (3) sandhi closes the handoff channel **before** the
  listen fd, so workers' `chan_recv` returns 0 and they exit rather than parking
  on a channel nobody will feed.

  `agnosai_serve` gains a third return meaning: **0 = asked to stop**, 1 =
  failed. Callers written against the old "any return is fatal" contract still
  behave correctly, since the failure value is unchanged.

  Installing signals is **not** folded into `agnosai_serve` — the suites call it
  with an unbindable address to prove a failed bind returns 1, and should not
  each leave a process-wide signal mask and a parked thread behind. A failed
  install warns and boots anyway: a server that cannot drain beats no server,
  and the oracle has no corresponding refusal.

  Verified live: both signals exit **0** in ~100 ms, and a request racing the
  shutdown still completes **200**. The in-process test signals itself with
  `sys_kill(sys_getpid(), SIGTERM)` and polls the flag — which doubles as proof
  the mask is installed, since without it that line would terminate the suite
  rather than fail an assertion.

- **`_agnosai_exit_process` now composes `sys_exit_group`** instead of a
  hand-rolled `syscall(SYS_EXIT_GROUP, …)`. The wrapper landed in cyrius 6.5.6
  from agnosai's filing. The `#ifdef CYRIUS_TARGET_LINUX` guard stays and is
  still load-bearing: `syscalls_linux_common.cyr` is included only by the two
  Linux target files, so the wrapper does not exist on agnos.

- **`main` binds and serves — M6 bite 16, and the first time the binary is a
  server.** `./build/agnosai` printed `agnosai ready` and exited; it now builds
  the event bus, orchestrator, tool registry and auth config, hands them to
  `agnosai_app_state_new`, and calls `agnosai_serve` on `INADDR_ANY:PORT`.
  Verified live rather than inferred: `/health` → 200 `{"status":"ok"}`,
  `/metrics` renders the registry, `/api/v1/tools` lists all four builtins, and
  `/api/v1/crews/{id}/stream` returns the deliberate 501 that bite 15c will
  replace.

  **`agnosai_serve_parse_port` is public while the rest of the env plumbing is
  not**, and the asymmetry is the point: nothing in `src/main.cyr` is reachable
  from a `.tcyr` — that file runs `main()` at include time — and the `u16` parse
  is the one piece with a real silent-divergence risk. Neither stdlib parser
  matches Rust, in *opposite* directions: `str_to_int` (`lib/str.cyr:280`) skips
  non-digits, so `"80a80"` answers 8080, and `atoi` (`lib/string.cyr:132`) stops
  at the first, answering 80. Neither can report failure at all. Both would bind
  a port the operator never asked for.

  Overflow is checked **per digit against 65535, never by capping digit count**.
  An earlier draft used a 5-digit cap and that is not `u16::from_str`'s grammar:
  Rust bounds the *value* through `checked_mul`/`checked_add` and accepts
  unbounded leading zeros, so `PORT=065535` is `Ok(65535)` there and the cap
  would have silently fallen back to 8080. Both spellings are now pinned by
  assertions and confirmed against the running binary.

  **`PORT=` set-but-empty does not fall through to `AGNOSAI_PORT`.** `.or_else`
  fires on `Err`, and `getenv` already distinguishes the cases — unset returns
  0, `FOO=` returns a non-zero pointer to `""` (`lib/io.cyr:621`), which is
  exactly `Err(NotPresent)` vs `Ok("")`. A `strlen(v) == 0` test in the first
  branch would have diverged. Eight port cases and nine auth cases were each run
  against the binary; the same distinction is why `AGNOSAI_JWT_PUBLIC_KEY=`
  builds no JwtConfig (the oracle's `.filter(|k| !k.is_empty())`).

  Two calls the oracle does not make, both deliberate. The resource budget is
  `agnosai_resource_budget_default()` rather than the `0` every existing suite
  passes as a shortcut — `agnosai_orchestrator_timeout_secs` dereferences it, so
  a `0` faults the moment a crew runs. And the JWT key is decoded eagerly via
  `agnosai_jwt_config_prepare`, which memoizes both outcomes and, because
  `_pem_init` guards its table with a plain non-atomic flag, removes a
  first-request race between worker threads. A bad key logs and boots anyway,
  matching the oracle's answer-500-per-request behaviour rather than refusing to
  start.


- **`SYS_EXIT` exits one thread, not the process — corrected before it could
  bite.** `src/main.cyr`'s epilogue ended `syscall(SYS_EXIT, code)`, which is
  `exit(2)`. That was harmless while `main` did nothing and stops being harmless
  the moment `agnosai_serve` runs: `sandhi_server_run_pooled` returns 1 from
  three places (`lib/sandhi.cyr:14192`, `:14203`, `:14213`), and the last two
  return into a process that already has up to 100 worker threads alive — some
  parked in `chan_recv`, some mid-request. Exiting only the main thread there
  leaves a process running with no acceptor: a hang, not a crash.

  Now `_agnosai_exit_process` calls `exit_group(2)` behind a target guard. The
  guard is load-bearing rather than decorative — the constant exists on every
  Linux target (231 x86_64, 94 on the aarch64 cross build named in
  `cyrius.cyml [release].cross_bins`) but **not on agnos**, which defines
  `SYS_EXIT` alone. Verified end to end: a privileged-port bind as a normal user
  returns and the process exits **1**, rather than hanging.

- **`tests/server_serve.tcyr` passed its bind-failure test for the wrong
  reason.** Three sites handed `str_from("192.0.2.1")` to `agnosai_serve`, whose
  `addr` is a **network-order IPv4 u32**, not a string — it goes unmodified to
  `sock_bind` → `sockaddr_in` → `store32(sa + 4, addr)` (`lib/net.cyr:99`). What
  actually landed in `sin_addr` was the low half of a 16-byte heap `Str` header
  pointer, so the assertions held against a garbage address while the comment
  claimed they held against TEST-NET-1. Replaced with `0x010200C0` (the same
  byte order as `INADDR_LOOPBACK()` = `0x0100007F`), and `agnosai_serve`'s doc
  now states the contract so the next caller cannot repeat it.

  This one mattered more than a tidy-up: the test is safe **only** because the
  bind fails before `thread_create`. A version that accidentally bound would
  spawn 100 workers, each reserving a 10 MiB request buffer, and then never
  return — `cyrius tests tests` would hang forever.

- **The dependency pins named versions nobody was building.** `cyrius.cyml`
  pinned **bote 3.2.1** and **kavach 3.9.3** while `lib/` held **bote 3.3.0** and
  **kavach 3.11.0** — both vendored bundles byte-identical to their upstream tag
  dists, verified by sha256 against `git show <tag>:dist/...`. The pins now say
  3.3.0 and 3.11.0, which is what was already being compiled and tested.

  **The mechanism matters more than the two numbers.** Every `[deps.NAME]`
  carries `path = "../NAME"` alongside `git` + `tag`, and **the local path
  wins**. A developer whose sibling checkout has moved ahead silently builds a
  version the manifest does not name; CI, which has no sibling checkouts,
  resolves the *tag* and builds something else. Here that was kavach **3.11.0
  locally against 3.9.3 in CI**, and neither skew was same-session: the
  `~/.cyrius/deps/kavach/3.11.0/` clone is dated **2026-08-02**, and `lib/`
  carried bote 3.3.0 from its **2026-07-31** release onward, while `state.md`
  went on recording it as "released and not yet pinned".

  This is the **inverse** of the sigil rule already on the books, and both are
  real: a stale tag can *overwrite* a newer folded copy (sigil's case) or be
  quietly *overridden* by a newer local path (kavach's case). The lockfile did
  not catch the second — `cyrius.lock` recorded 3.9.3's sha256 against a 3.11.0
  file on disk and nothing surfaced it, because every other gate reads `src/`
  and the build compiles whatever bytes `lib/` holds.

  **Neither bump changes any path agnosai executes**, which is why the tests
  stayed green through a version skew nobody had noticed. kavach 3.10.0/3.11.0
  are `--agnos` target build fixes — nine additive `kv_*` shims (`kv_unlink`,
  `kv_rmdir`, `kv_waitpid`, `kv_getgid`, `kv_lstat`, `kv_fork`, `kv_dup2`,
  `kv_execve`, `kv_setsid`) — and agnosai calls only `score_agent`,
  `score_agent_with_tools`, `sandbox_display` and `sandbox_strength`, none of
  which the diff touches. bote 3.3.0 adds `dispatcher_set_server_info` and is
  additive by construction: an unconfigured dispatcher emits the pre-3.3.0 wire
  byte for byte. Duplicate-fn warnings held at **35**, all lib-vs-lib; 57 suites
  and coverage 100% unchanged.


- **A cyclic DAG ratcheted the `crews_active` gauge upward forever.** Introduced
  and caught within this change: the first cut recorded `crew_started` before the
  DAG-cycle early return, which exits without recording a completion — so one
  malformed spec permanently inflated the gauge for the life of the process. The
  error path now balances it. It still emits no `crew_completed` **event** (a
  cyclic spec is an error, not a crew that failed); the event stream and the
  gauge are different contracts and only the event one is suppressed. Pinned by a
  mutation-verified assertion in `tests/orch_crew_runner.tcyr` — deleting the
  balancing call fails it.
- **`agnosai_chan_push_lossy` reported a dropped event as delivered.** When the
  post-eviction retry also came back full — another producer refilled the ring
  between the two calls — it fell through to `return 1`, which the contract
  defines as "stored, after evicting the oldest". Nothing was stored. The single
  consumer of that distinction is `agnosai_event_sender_send`, whose whole job is
  counting what a slow SSE reader lost, so the one caller that exists to measure
  loss was told a lost message had landed.

  New `AGNOSAI_CHAN_DROPPED` (-2), distinct from `AGNOSAI_CHAN_CLOSED` (-1) and
  from 1. `agnosai_event_sender_send` now counts **both** 1 and
  `AGNOSAI_CHAN_DROPPED` toward `AGN_ER_LAGGED`, because a message was lost
  either way — testing only `== 1` would have under-reported the worse of the two.

  Only reachable with concurrent producers on one topic: a single producer always
  wins the slot its own eviction freed. That is exactly why it was worth writing
  down rather than discovering later from a lag count that did not add up. Pinned
  by four asserts in `tests/orch_pubsub.tcyr` on the contract (the three non-zero
  returns are mutually distinct, and the ordinary success path still returns 0).

- **server_serve — sandhi's accessors return NUL-terminated cstrings, not `Str`,
  and the adapter passed all three straight through.** `sandhi_server_get_method`,
  `_get_path`, and `_find_header` each `alloc_via` a NUL-terminated copy
  (`lib/sandhi.cyr:12358-12427`); reading one as a `Str` reads its first eight
  bytes as a data pointer and the next eight as a length. The method compare
  silently answered "no method" — **every request would have been 405** — and
  `str_len(path)` returned 21815634, faulting in the router. Fixed by wrapping
  with `str_from_a` (which borrows the bytes, no copy) and comparing the method
  with `streq` rather than `str_eq`. All three are mutation-verified: reverting
  any one fails the end-to-end suite.
- **server_router — path matching re-split every pattern on every request.** The
  matcher built a vec of `Str` for both sides, so each of the 18 patterns was
  re-segmented per call and a request matching nothing walked all of them.
  Rewritten to walk both sides in place, allocating only the captured parameter
  on the one pattern that matches. A segment must now match in full — mutation
  testing found `/heal` reaching `/health` had no covering test.
- **The doc, lint, and coverage gates were never in CI.** `.github/workflows/ci.yml`
  ran the symbol check, build, and test — not `fmt`, `lint`, `doc`, `vet`,
  `deny`, or `coverage`, despite CLAUDE.md's work loop specifying them at steps
  2 and 6 and calling coverage out as "its own CI step". The drift was real:
  **31 undocumented public symbols** across five modules and four untracked lint
  deferrals, none of which any pipeline would have reported. All 35 fixed, and
  `scripts/check-clean.sh` now gates the class — verified to fail on an
  undocumented symbol, a formatting violation, and an untracked deferral.
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
  `src/server/ssrf.cyr:39` and `src/server/output_filter.cyr:140` — and is the 36th
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

- **Four duplicated top-level constants in `src/`, three with different values —
  and the compiler warns about none of them.** Found by a flat-namespace audit
  run as the prerequisite for porting `mcp.rs`.

  Cyrius has one global symbol table and last-definition-wins. It emits
  `warning: duplicate fn` for a repeated **fn** and is **silent** for a repeated
  `var` or enum member — so `grep "duplicate fn"` on the build log, the obvious
  check, structurally cannot catch this class.

  | constant | | |
  |---|---|---|
  | `AGN_AC_SIZE` | `server_auth.cyr` = **24** | `orch_audit.cyr` = **48** |
  | `AGN_CE_SIZE` | `server_sse.cyr` = **24** | `orch_plan_cache.cyr` = **32** |
  | `AGN_RR_SIZE` | `server_routes.cyr` = **24** | `tools_remote_registry.cyr` = **40** |
  | `AGN_REQ_SIZE` | `core_resource.cyr` = 40 | `llm_hoosh.cyr` = 40 |

  Three of the four are **struct sizes passed straight to `alloc()`**. Nothing
  misbehaved, but only by accident of include order: each file's own `alloc()`
  happens to be parsed after its own definition and before the redefinition.
  Reordering `src/main.cyr`'s includes, or adding a module that used one of these
  names, would have silently under-allocated a heap struct with no diagnostic.
  All eight now carry module-unique names; behaviour is unchanged (52/52 suites,
  837/837 coverage, before and after).

- **`_agnosai_is_digit` hoisted into `src/units.cyr`.** It was defined
  byte-identically in both `server_ssrf.cyr` and `server_output_filter.cyr` — a
  silent last-definition-wins pair, benign only while the bodies agreed. This was
  documented as "hoist at the next touch" and the new gate forced the issue.
  agnosai's own duplicate-fn warnings are now **zero** (build total 36 → 35; the
  remaining 35 are all lib-vs-lib).

- **`src/main.cyr`'s entry-point `var r` renamed** to `_agnosai_exit_code`. A
  bare single-letter top-level `var` is a global in the flat namespace.

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

### Security

- **server_auth — six defects found by an adversarial review of the first cut of
  the JWT half, all fixed and all pinned by a test that fails if the fix is
  removed** (verified by mutation, not assumed). Recorded in full because five of
  the six were introduced in this release and would otherwise leave no trace.
  - **Unauthenticated heap exhaustion, ~53× amplification.** The header was
    parsed *before* the signature was verified, and `bayan_json_v_parse_buf`
    builds its tree on the no-free global bump. A 1,164-byte header segment
    leaked **62,248 bytes per request** on a path returning 401 — no credential
    required, and nothing reclaims it. Fixed by verifying the signature first,
    which is cryptographically safe here because the algorithm is never *chosen*
    from the header: this path always runs RSASSA-PKCS1-v1.5/SHA-256 with the
    configured key, so a token declaring `alg:none` still needs a valid RSA
    signature to get anywhere. Measured after: **32 bytes, flat**, independent of
    header size, and pinned by `_t_jwt_preauth_allocation_is_bounded`.
  - **A second unauthenticated leak, 536 bytes/request.** `agnosai_jwt_config_prepare`
    memoized success but not failure, so `AGNOSAI_JWT_KEY_BAD` was written and
    never read: a PEM that base64-decoded and then failed RSA parsing was
    re-decoded on every request. One mistyped `AGNOSAI_JWT_PUBLIC_KEY` was enough.
  - **`exp` overflow, fail-open.** bayan's `_jp_atoi` computes `n = n*10 + digit`
    in wrapping i64 with no overflow detection and still tags the result
    `JTAG_INT`, so the `is_int` gate did not catch it. A payload carrying
    `"exp":20000000000000000000` wrapped to `1553255926290448384` — a
    year-51-billion timestamp — and the token was **accepted** where the oracle
    answers 401. Now range-guarded to `[0, 253402300799]`.
  - **Weak keys accepted.** The oracle verifies through ring's
    `RSA_PKCS1_2048_8192_SHA256`, which refuses any modulus under 2048 bits;
    sigil enforces no minimum at all. A **512-bit** key was accepted and its
    tokens verified. Now floored at 2048-bit.
  - **A zero clock silently disabled expiry.** `clock_epoch_secs` is documented
    to return 0 when the RTC is unreadable — "unknown", not 1970 — and on the
    agnos target it is a bare `sys_time_unix` with no retry. Fed through, the
    expiry test became `exp < -60`, false for every non-negative `exp`. Now
    refused with a 500. Note this one initially had a test that *looked* right
    and caught nothing: on a host with a working clock the branch is unreachable,
    so the guard was split into `agnosai_auth_check_clocked` to make it drivable.
  - **Claim types unchecked.** The oracle deserializes the payload into a typed
    `Claims` before validating, so any type mismatch is a 401; the port read it
    untyped and accepted `"sub":123`, a negative `iat`, and a non-string `scope`
    or `iss`. Now type-checked against every field the oracle declares.

  Still divergent and **documented rather than fixed**: duplicate JSON members
  resolve first-wins where serde errors; unknown JWS header members are not
  type-checked; the modulus ceiling is 4096-bit against the oracle's 8192-bit,
  because sigil's `_rsa_recover_em` refuses anything wider.

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

- **tools** (M4, Phase 3, in progress) — `src/tools/mod.cyr` hub plus two submodules:
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
- **orch** (M5, Phase 4, in progress) — `src/orchestrator/mod.cyr` hub. Two modules so far:
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

### Performance

- **The whole `core` group builds on a per-request arena — a 10-agent, 10-task
  crew serialization drops 44,032 → 0 bytes on the global bump**, and runs 11%
  faster (171 → 152 µs). `core/json`, `core/task`, `core/resource`, `core/agent`,
  `core/message` and `core/crew` are threaded end to end: 27 `_a` forms, each with
  the bare name delegating through `default_alloc()`.

  Every `_a` form is pinned as **agreeing byte-for-byte with its global twin** —
  that is the correctness claim the whole conversion rests on, and it is the only
  assertion that would catch a substitution which silently changed a *value*
  rather than just where it was allocated.

  **Two bugs found doing it, both worth recording:**

  1. **`fn agnosai_agent_to_value_a(a, a)` — Cyrius accepted a duplicate parameter
     name silently.** The original parameter was already `a` (the agent pointer),
     and prepending an allocator also called `a` compiled clean; every
     `load64(a + AGN_AGENT_OFFSET)` then read the *allocator* and the suite
     SIGSEGV'd with no assertion output. The conversion now picks a non-colliding
     allocator name. This is the sharp edge of one flat namespace plus no arity or
     shadowing diagnostic — a rename that looks mechanical is not.
  2. **`map_keys` is the recurring residue.** It materialises a key vec through
     `vec_new()` on the global bump and has no `_a` form, so it survives every
     other substitution and silently caps the win. It appeared again in
     `agnosai_task_dag_to_value` and in crew's two cost/int map helpers after the
     first fix. `src/core/json.cyr` now exposes `_agnosai_map_slots` /
     `_slot_live` / `_slot_key` / `_slot_val` over the documented hashmap layout
     (`lib/hashmap.cyr:22-35`), guarded on `key_type == 2`, so the layout has one
     place to be wrong instead of four. Expect it in any module that serialises a
     map.

- **A task response can now be built entirely on a per-request arena — 1944 → 0
  bytes on the global bump.** Toolchain pin **6.5.4 → 6.5.5**, which folds
  **bayan 1.4.0** and its completed `_a` JSON surface. `lib/` matches the pin
  exactly (0 of 99 stdlib files differ, 0 drift warnings).

  Measured same-box, `agnosai_task_to_json` over 20k iterations:

  | path | bytes/response | ns/response |
  |---|---|---|
  | global (back-compat wrapper) | 1792 | 7758 |
  | arena + `reset_via` per response | **0** | **6922** |

  Wire is **byte-identical** between the two paths — asserted, not assumed.

  Threaded: the five allocating helpers in `src/core/json.cyr`, the three
  `*_to_wire` spellings, `agnosai_task_to_value` and `agnosai_task_to_json`, each
  as an `_a` body with the bare name delegating through `default_alloc()` — the
  stdlib's own convention (`str_from_a`, `alloc_via`).

  **The last 152 bytes were `map_keys`.** After everything else moved,
  `_agnosai_map_to_value` still called it, and it materialises a key vec through
  `vec_new()` on the global bump with no `_a` form to thread. It now walks the
  map's slots directly — that is the **documented public layout**
  (`lib/hashmap.cyr:22-35`: header `{entries_ptr, capacity, count, key_type}`,
  slot `{key_ptr, value, state}`, 24 bytes, `state == 1` occupied), not a reach
  into internals, and it skips the intermediate vec entirely so it helps the
  global path too — that is why the non-arena row above reads 1792 rather than
  the previously measured 1944. Guarded on `key_type == 2`: the u64 map has a
  16-byte slot and no state field, and the same header notes `map_keys` /
  `map_values` / `map_iter` do not work on it either.

  Pinned by assertions in `tests/core_task.tcyr` and **mutation-verified twice** —
  putting a single key Str back on the global bump fails the zero-growth
  assertion, and so does restoring the `map_keys` call.

  This is one module. 48 more `*_to_value` fns and ~645 constructor call sites
  remain; `core/json` + `core/task` was the first bite because it is the one with
  an existing benchmark to measure against.
- **`src/order.cyr` now delegates to the stdlib sort — 184 → 98 lines.**
  `vec_sort_by` / `vec_select_nth` shipped in cyrius **6.5.4**, closing agnosai's
  own filing (`2026-07-28-agnosai-no-nlogn-sort-in-stdlib`). The vendored
  heapsort, Hoare quickselect, median-of-3, partition and swap are deleted.

  Measured same-box before/after (`benches/order.bcyr`):

  | benchmark | vendored | stdlib | |
  |---|---|---|---|
  | `sort_100k` | 79.6 ms | **20.3 ms** | 3.9× |
  | `sort_10k` | 6.30 ms | **1.71 ms** | 3.7× |
  | `sort_100k_already_sorted` | 79.1 ms | **3.31 ms** | **23.9×** |
  | `three_percentiles_100k` | 10.7 ms | **7.81 ms** | 1.4× |
  | `select_nth_100k` | 6.89 ms | **5.09 ms** | 1.4× |

  The already-sorted row is a difference in kind, not degree: heapsort's worst
  case equals its average, so it had no fast path at all. Introsort checks for
  pre-sorted input first, and a latency vector that arrives roughly ordered —
  common — now costs a scan instead of a full sort. `builtin/load_testing` is the
  consumer that pays this on every run.

  **The public API and its bounds contract are unchanged, deliberately.** This is
  a wrapper, not a rename: `vec_select_nth` **aborts the process** (`_vec_die()`)
  on `k < 0` or `k >= len`, where `agnosai_select_nth` returns 0 — a contract
  three assertions in `tests/order.tcyr` already pinned (empty vec, past-the-end,
  negative k). An empty latency vector is an ordinary state for a load test that
  recorded no samples, not a reason to kill the server. The guards stay in front;
  only the sorting is delegated.

  Bench labels lost their now-wrong `_heapsort` suffix. 57 suites green, all 48
  order assertions unchanged.
- **86 `str_eq(x, str_from("lit"))` comparisons → `str_eq_cstr(x, "lit")`.** Each
  of those sites allocated a fresh 16-byte `Str` header on the **no-free global
  bump** just to compare against a compile-time constant, and never released it.
  Measured on the `core/task` wire-decode path (same box, same toolchain, same
  session, `HEAD` vs working tree, 200k rounds of three decodes):

  | | ns / 3-decode round | bytes / round |
  |---|---|---|
  | before | 482 | 128 |
  | after | **213** | **0** |

  2.26× faster and the leak is gone outright, not reduced.

  **This is a Rust reflex, not a Cyrius one.** In Rust `"medium"` is a zero-cost
  `&'static str` and `s == "medium"` allocates nothing, so the shape is free there
  and invisible on review. In Cyrius `str_from` is a heap constructor and `alloc()`
  never frees an individual allocation. The density says the same thing: 910
  `str_from("` sites in 19,671 lines is 4.6 per 100 lines, against **0.064** in the
  cyrius stdlib (96 in 150,822) and 0.31 in vidya.

  **`str_eq_cstr` already existed** (`lib/str.cyr:617`) — length guard then
  `memeq`, zero allocation, and it derives the literal's length with `strlen` so
  there are no hand-written lengths to get wrong. No local helper was needed; the
  gap was in reading the stdlib, not in the stdlib. Equivalence was proved over
  the edge cases before any site was touched — exact match, prefix either way,
  both-empty, either-empty, and a `Str` carrying an embedded NUL (where `Str`'s
  explicit length and a cstr's NUL terminator could have disagreed): 8/8 identical,
  0 bytes allocated across 2000 calls.

  Rewritten by a balance-scanning pass rather than a regex — three sites have a
  call in the first argument and one of those (`str_new(d + start, len - start)`,
  `llm/hoosh.cyr:594`) contains a comma that a naive `[^,]*` pattern would have
  split through.

  `src/` drops from 910 `str_from("` sites to 824. The remaining classes are
  separate bites: 149 `return str_from("lit")` constant returns, and the in-loop
  hoists. The 380 sites under `tests/` are deliberately left — a test binary is
  short-lived, so the leak is inert there, and rewriting assertions is churn
  against no measurable cost.

- **Route resolution: 4352 → 48 bytes per request (−99%).** A full-table miss was
  the expensive case at 4304 B, twelve times a hit; it is now 352 B end to end,
  the same as a hit. `/health` end to end: 720 → 352 B (−51%); `/api/v1/tools`:
  4808 → 1920 B (−60%). Measured with `alloc_used()` over 64 iterations after
  warm-up. Writing bite 15b's allocation test is what surfaced it — routing cost
  six times the handler it was dispatching to.

- **server_auth (JWT)**: `auth_jwt_verify_ok` **3.31 ms**, of which the raw
  `rsa_pkcs1v15_verify_sha256` is **3.29 ms** — measured in isolation, so the
  port's own parsing, base64 and JSON work is the remaining ~20 µs.
  `auth_jwt_key_prepare` is **10.7 µs**, which is the per-request cost the oracle
  pays and this port does not.

  **The RSA figure is a sigil finding, not an agnosai one, and it is large.**
  OpenSSL on this box does an RSA-2048 verify in **14 µs** (`openssl speed
  rsa2048`, 70,445/s) — sigil is **~235× slower**. Part is inherent (a portable
  bignum against hand-tuned assembly), but part is not:
  `bn_mont_modexp` (`lib/sigil.cyr:10790`) runs a **constant-time always-multiply
  ladder over the full `exp_blen * 8` bit range**, with no leading-zero skip —
  where the sibling `bn_modexp` (`:10508`) does locate the high bit first. For
  the public exponent 65537 that is 24 iterations × 2 multiplies = 48, against
  the ~17 a conditional ladder needs, and the call site's own comment
  (`lib/sigil.cyr:17632-17636`) says the operands are public and "Montgomery is
  used purely for speed, not for the side-channel posture (no secret to protect
  on the verify path)" — so it is paying ~2.8× for protection it states it does
  not need. Worth filing against sigil.

  **The operational consequence is agnosai's**: at 3.3 ms a core sustains only
  ~300 JWT verifies/second. `auth_jwt_reject_bad_alg` is **3.29 ms** — the same
  cost — because the `alg` check deliberately sits *after* signature
  verification, so a rejected token pays the modexp too. An earlier ordering
  rejected in 3.75 µs but parsed attacker-controlled JSON before authenticating,
  which measured as a ~53x unauthenticated heap amplification (see **Security**).
  Permanent memory exhaustion is the worse failure, so the CPU cost is accepted
  and the answer to a flood is a rate limiter rather than check ordering. That
  makes `rate_limit.rs` materially more important to M6 than its never-mounted
  status in `server/mod.rs` suggests.
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

#### Fixed
- **Four duplicated top-level constants in `src/`, three with different values —
  and the compiler warns about none of them.** Found by a flat-namespace audit
  run as the prerequisite for porting `mcp.rs`.

  Cyrius has one global symbol table and last-definition-wins. It emits
  `warning: duplicate fn` for a repeated **fn** and is **silent** for a repeated
  `var` or enum member — so `grep "duplicate fn"` on the build log, the obvious
  check, structurally cannot catch this class.

  | constant | | |
  |---|---|---|
  | `AGN_AC_SIZE` | `server_auth.cyr` = **24** | `orch_audit.cyr` = **48** |
  | `AGN_CE_SIZE` | `server_sse.cyr` = **24** | `orch_plan_cache.cyr` = **32** |
  | `AGN_RR_SIZE` | `server_routes.cyr` = **24** | `tools_remote_registry.cyr` = **40** |
  | `AGN_REQ_SIZE` | `core_resource.cyr` = 40 | `llm_hoosh.cyr` = 40 |

  Three of the four are **struct sizes passed straight to `alloc()`**. Nothing
  misbehaved, but only by accident of include order: each file's own `alloc()`
  happens to be parsed after its own definition and before the redefinition.
  Reordering `src/main.cyr`'s includes, or adding a module that used one of these
  names, would have silently under-allocated a heap struct with no diagnostic.
  All eight now carry module-unique names; behaviour is unchanged (52/52 suites,
  837/837 coverage, before and after).

- **`_agnosai_is_digit` hoisted into `src/units.cyr`.** It was defined
  byte-identically in both `server_ssrf.cyr` and `server_output_filter.cyr` — a
  silent last-definition-wins pair, benign only while the bodies agreed. This was
  documented as "hoist at the next touch" and the new gate forced the issue.
  agnosai's own duplicate-fn warnings are now **zero** (build total 36 → 35; the
  remaining 35 are all lib-vs-lib).

- **`src/main.cyr`'s entry-point `var r` renamed** to `_agnosai_exit_code`. A
  bare single-letter top-level `var` is a global in the flat namespace.

### Added
- **`scripts/check-symbols.sh` + a CI gate.** Two rules the compiler cannot
  enforce: no name defined twice in `src/` across **all** definition kinds (fn,
  var, enum member), and every top-level symbol `agnosai_*`/`_`-prefixed. The
  second rule closes the entire ~180-name unprefixed export surface of
  `lib/bote-core.cyr` at once, with no denylist to maintain. Runs before Build.

### Security — Prompt Injection & Tool Allow-Lists
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

### Fixed
- **Four duplicated top-level constants in `src/`, three with different values —
  and the compiler warns about none of them.** Found by a flat-namespace audit
  run as the prerequisite for porting `mcp.rs`.

  Cyrius has one global symbol table and last-definition-wins. It emits
  `warning: duplicate fn` for a repeated **fn** and is **silent** for a repeated
  `var` or enum member — so `grep "duplicate fn"` on the build log, the obvious
  check, structurally cannot catch this class.

  | constant | | |
  |---|---|---|
  | `AGN_AC_SIZE` | `server_auth.cyr` = **24** | `orch_audit.cyr` = **48** |
  | `AGN_CE_SIZE` | `server_sse.cyr` = **24** | `orch_plan_cache.cyr` = **32** |
  | `AGN_RR_SIZE` | `server_routes.cyr` = **24** | `tools_remote_registry.cyr` = **40** |
  | `AGN_REQ_SIZE` | `core_resource.cyr` = 40 | `llm_hoosh.cyr` = 40 |

  Three of the four are **struct sizes passed straight to `alloc()`**. Nothing
  misbehaved, but only by accident of include order: each file's own `alloc()`
  happens to be parsed after its own definition and before the redefinition.
  Reordering `src/main.cyr`'s includes, or adding a module that used one of these
  names, would have silently under-allocated a heap struct with no diagnostic.
  All eight now carry module-unique names; behaviour is unchanged (52/52 suites,
  837/837 coverage, before and after).

- **`_agnosai_is_digit` hoisted into `src/units.cyr`.** It was defined
  byte-identically in both `server_ssrf.cyr` and `server_output_filter.cyr` — a
  silent last-definition-wins pair, benign only while the bodies agreed. This was
  documented as "hoist at the next touch" and the new gate forced the issue.
  agnosai's own duplicate-fn warnings are now **zero** (build total 36 → 35; the
  remaining 35 are all lib-vs-lib).

- **`src/main.cyr`'s entry-point `var r` renamed** to `_agnosai_exit_code`. A
  bare single-letter top-level `var` is a global in the flat namespace.

### Added
- **`scripts/check-symbols.sh` + a CI gate.** Two rules the compiler cannot
  enforce: no name defined twice in `src/` across **all** definition kinds (fn,
  var, enum member), and every top-level symbol `agnosai_*`/`_`-prefixed. The
  second rule closes the entire ~180-name unprefixed export surface of
  `lib/bote-core.cyr` at once, with no denylist to maintain. Runs before Build.

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

### Fixed
- **Four duplicated top-level constants in `src/`, three with different values —
  and the compiler warns about none of them.** Found by a flat-namespace audit
  run as the prerequisite for porting `mcp.rs`.

  Cyrius has one global symbol table and last-definition-wins. It emits
  `warning: duplicate fn` for a repeated **fn** and is **silent** for a repeated
  `var` or enum member — so `grep "duplicate fn"` on the build log, the obvious
  check, structurally cannot catch this class.

  | constant | | |
  |---|---|---|
  | `AGN_AC_SIZE` | `server_auth.cyr` = **24** | `orch_audit.cyr` = **48** |
  | `AGN_CE_SIZE` | `server_sse.cyr` = **24** | `orch_plan_cache.cyr` = **32** |
  | `AGN_RR_SIZE` | `server_routes.cyr` = **24** | `tools_remote_registry.cyr` = **40** |
  | `AGN_REQ_SIZE` | `core_resource.cyr` = 40 | `llm_hoosh.cyr` = 40 |

  Three of the four are **struct sizes passed straight to `alloc()`**. Nothing
  misbehaved, but only by accident of include order: each file's own `alloc()`
  happens to be parsed after its own definition and before the redefinition.
  Reordering `src/main.cyr`'s includes, or adding a module that used one of these
  names, would have silently under-allocated a heap struct with no diagnostic.
  All eight now carry module-unique names; behaviour is unchanged (52/52 suites,
  837/837 coverage, before and after).

- **`_agnosai_is_digit` hoisted into `src/units.cyr`.** It was defined
  byte-identically in both `server_ssrf.cyr` and `server_output_filter.cyr` — a
  silent last-definition-wins pair, benign only while the bodies agreed. This was
  documented as "hoist at the next touch" and the new gate forced the issue.
  agnosai's own duplicate-fn warnings are now **zero** (build total 36 → 35; the
  remaining 35 are all lib-vs-lib).

- **`src/main.cyr`'s entry-point `var r` renamed** to `_agnosai_exit_code`. A
  bare single-letter top-level `var` is a global in the flat namespace.

### Added
- **`scripts/check-symbols.sh` + a CI gate.** Two rules the compiler cannot
  enforce: no name defined twice in `src/` across **all** definition kinds (fn,
  var, enum member), and every top-level symbol `agnosai_*`/`_`-prefixed. The
  second rule closes the entire ~180-name unprefixed export surface of
  `lib/bote-core.cyr` at once, with no denylist to maintain. Runs before Build.

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

### Fixed
- **Four duplicated top-level constants in `src/`, three with different values —
  and the compiler warns about none of them.** Found by a flat-namespace audit
  run as the prerequisite for porting `mcp.rs`.

  Cyrius has one global symbol table and last-definition-wins. It emits
  `warning: duplicate fn` for a repeated **fn** and is **silent** for a repeated
  `var` or enum member — so `grep "duplicate fn"` on the build log, the obvious
  check, structurally cannot catch this class.

  | constant | | |
  |---|---|---|
  | `AGN_AC_SIZE` | `server_auth.cyr` = **24** | `orch_audit.cyr` = **48** |
  | `AGN_CE_SIZE` | `server_sse.cyr` = **24** | `orch_plan_cache.cyr` = **32** |
  | `AGN_RR_SIZE` | `server_routes.cyr` = **24** | `tools_remote_registry.cyr` = **40** |
  | `AGN_REQ_SIZE` | `core_resource.cyr` = 40 | `llm_hoosh.cyr` = 40 |

  Three of the four are **struct sizes passed straight to `alloc()`**. Nothing
  misbehaved, but only by accident of include order: each file's own `alloc()`
  happens to be parsed after its own definition and before the redefinition.
  Reordering `src/main.cyr`'s includes, or adding a module that used one of these
  names, would have silently under-allocated a heap struct with no diagnostic.
  All eight now carry module-unique names; behaviour is unchanged (52/52 suites,
  837/837 coverage, before and after).

- **`_agnosai_is_digit` hoisted into `src/units.cyr`.** It was defined
  byte-identically in both `server_ssrf.cyr` and `server_output_filter.cyr` — a
  silent last-definition-wins pair, benign only while the bodies agreed. This was
  documented as "hoist at the next touch" and the new gate forced the issue.
  agnosai's own duplicate-fn warnings are now **zero** (build total 36 → 35; the
  remaining 35 are all lib-vs-lib).

- **`src/main.cyr`'s entry-point `var r` renamed** to `_agnosai_exit_code`. A
  bare single-letter top-level `var` is a global in the flat namespace.

### Added
- **`scripts/check-symbols.sh` + a CI gate.** Two rules the compiler cannot
  enforce: no name defined twice in `src/` across **all** definition kinds (fn,
  var, enum member), and every top-level symbol `agnosai_*`/`_`-prefixed. The
  second rule closes the entire ~180-name unprefixed export surface of
  `lib/bote-core.cyr` at once, with no denylist to maintain. Runs before Build.

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
