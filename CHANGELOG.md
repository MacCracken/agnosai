# Changelog

All notable changes to AgnosAI will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [2.0.1] — 2026-08-18

### Changed — Cyrius pin 6.5.21 → 6.5.27

Part of the ecosystem-wide ML/AI-arc toolchain realign. `cyrius lib sync --full` re-vendored the
version-matched stdlib snapshot; suite **7940/7940 across 98 suites**, unchanged.

⚠ **The pin move is what broke `scripts/check-clean.sh`**, exactly as the sigil section below
predicts. The 6.5.21 fold shipped sigil **3.12.7**, which is why `[deps.sigil] tag = "3.12.7"`
matched it; 6.5.27's fold ships **3.12.9**, and since `cyrius deps` overlays a declared dep's copy
onto the fold on every resolve, the old tag put a stale `lib/sigil.cyr` back over the synced one:
`lib: sigil.cyr differs from the 6.5.27 snapshot`. Fixed by moving the tag to 3.12.9 — now
byte-identical to the snapshot, 107 of 108 folded files match.

⛔ **The thin-sigil selection described below is NOT in `cyrius.cyml`.** `[deps.sigil]` still reads
`modules = ["dist/sigil.cyr"]`. Every other change in this release landed — the manifest is 66 lines
with zero comments, `scripts/lock-refresh.sh`, `scripts/hooks/pre-commit` and the *Lockfile is
honest* CI step are all gone — but the module selection is not, so the tag remains coupled to the
toolchain fold and **this failure will recur on the next pin bump**. The tag bump is a workaround,
which is precisely what that section set out to avoid. One likely reason it was never applied: the
eight modules it names are **not sufficient** — agnosai also calls `sha256_hex`, which lives in
`src/hex.cyr`. The working set is nine: `crypto_scratch`, `mul64`, `bignum`, `bigint_ext`, `sha_ni`,
`sha256`, `hex`, `hmac`, `rsa` — matching the pattern libro and kybernet already use.

- **`[deps.kavach]` `3.11.13` -> `3.11.14`** and **`[deps.tyche]` `1.0.0` -> `1.0.1`** (tyche's frozen
  1.x surface), alongside the sigil bump above.
- **`[deps.ai-hwaccel]` `2.3.16` -> `2.3.17`.**

### Changed — kavach 3.11.12 → 3.11.14, sigil 3.12.7 → 3.12.9, tyche 1.0.0 → 1.0.1

**sigil 3.12.9 is the one with teeth here.** It moves the RSA sign, blind and
CRT workspace off shared `cbank()` lanes into function-scope locals, and makes
secret zeroization per-CALL rather than per-lane. `agnosai_serve` verifies RS256
bearer tokens under `sandhi_server_run_pooled`, whose workers are **real OS
threads**, so on the old pin two concurrent verifies could alias a lane — and
the per-lane wipe could zero a sibling's in-flight secret mid-sign.

**kavach 3.11.13** renames its fourteen bare `err_*` constructors to
`kavach_err_*`. agnosai calls none of them — verified across `src/` and
`tests/` for `err_*`, `syserr_*`, `wrap_syscall`, `is_syscall_err` and
`result_print_err` — so it is a clean bump.

### Changed — sigil is a THIN dependency, not the whole library

`[deps.sigil]` took `dist/sigil.cyr` — **1,084 KB and 1,163 functions** — for
the three symbols agnosai actually calls: `rsa_pkcs1v15_verify_sha256` and
`rsa_pubkey_from_der` (RS256, `src/server/auth.cyr`) and `hmac_sha256`. It now
takes eight source modules (~165 KB): `crypto_scratch`, `mul64`, `bignum`,
`bigint_ext`, `sha_ni`, `sha256`, `hmac`, `rsa`.

This is the pattern the tree already used and agnosai alone ignored — libro and
kybernet both select thin sigil sub-bundles, and the manifest even carried a
note that a consumer taking `dist/sigil.cyr` builds with
`warning: large static data (10794336 bytes)`.

It also removes a CI failure at the root rather than working around it.
`sigil.cyr` is the one dep file that is **also** in the cyrius stdlib fold, and
`check-clean.sh` requires every folded file to byte-match the snapshot. While
this dep pulled `dist/sigil.cyr`, `cyrius deps` overlaid the tag's copy onto the
fold's, so any tag but the fold's 3.12.7 produced
`lib: sigil.cyr differs from the 6.5.21 snapshot`. Selecting source modules
means `deps` never writes `lib/sigil.cyr` at all: the fold keeps it, the gate is
satisfied, and the tag moves independently of the toolchain — so 3.12.9's
thread-safety fix lands for real.

### Removed — the cyrius.lock pre-commit dance

CI's *Lockfile is honest* step diffed the committed lock against its own clean
resolution and failed on any difference. That required a developer to
hand-produce a byte-equal lock while every local `cyrius deps` / `build` /
`tests` rewrote it with zero commit pins — two requirements in direct conflict.
The lock landed wrong **five times**, and `scripts/lock-refresh.sh` plus a
`scripts/hooks/pre-commit` guard had grown to paper over it.

All three are gone. **CI resolves the lock and publishes it as an artifact.**
The protections that carried real signal are untouched: `cyrius deps` fails on
an unresolvable tag, `deps --verify` confirms the lock matches `lib/`, and the
lib-snapshot diff catches a dep downgrading a folded module.

### Changed — `cyrius.cyml` is configuration again

200 lines, 132 of them comments, down to 76 with none. The rationale lives in
this file and `docs/development/`.

### Fixed — `durable_state` segfaulted reading a snapshot path that is a directory

**orchestrator/durable_state** — `_agnosai_read_file_exact` classified a
directory as `-EISDIR` only *after* `xlseek(fd, 0, 2)` returned a negative,
documented as the measured behaviour: "opening a directory succeeds, the seek
then fails `-EINVAL`". That measurement was taken on a **tmpfs `/tmp`** and does
not generalise. On **ext4, `lseek(SEEK_END)` on a directory fd succeeds and
returns i64 MAX** (9223372036854775807), so the check never fired,
`alloc(size + 1)` overflowed to a negative request and returned 0, and
`store8(buf + size, 0)` wrote at offset 2^63-1 — SIGSEGV.

The check is now hoisted **before the open**, which is the placement
`definitions/loader.cyr` already used and documented (its comment named this
function as the one that did not). The allocation is checked before it is
written through, so a failed `alloc` reports `-ENOMEM` instead of a wild store.

⚠ **This only ever crashed on CI**, whose `/tmp` is ext4 while a dev box's is
tmpfs. `tests/orch_durable_state.tcyr` passed 300 consecutive local runs — clean
and with leftover state — because tmpfs takes the `-EINVAL` path. Pointing the
suite's root at an ext4 path reproduced it on the first attempt: old code
exit 139 (signal 11), fixed code 82 passed / 0 failed.

### Fixed — a >=2 GiB definition file crashed the whole-file reader

**definitions/loader** — the *reachable* instance of the class above, and the
one the first sweep missed. `_agnosai_loader_read` wrote through
`alloc(size + 1)` without checking it. `alloc` fails closed above `ALLOC_MAX`
(2 GiB) and returns 0, so `store8(buf + size, 0)` wrote at address `size` —
unmapped — for a SIGSEGV. **No privilege needed**: any regular file of
2147483648 bytes or more, and `agnosai_load_from_file` reads *before* it checks
the extension, so the filename need not even look loadable. The same line
faulted for an ordinary-sized file whenever the backing mmap failed. This is the
third of the tree's three whole-file readers; the other two were hardened first
and this one was missed in that same commit, while its comment block was open.

### Hardened — allocation guards in `definitions/packaging`

`_agnosai_pkg_read_member` now rejects a negative `zip_entry_size` and checks
its allocation, and `agnosai_package_export` checks the buffer it hands to
`zip_writer_init`, which validates `dst_cap` but never null-checks `dst`.

⚠ **Correction to an earlier draft of this entry**, which claimed the negative
arm was a live heap-corruption bug on a caller-supplied path. It is not: both
call sites take their index from `_agnosai_pkg_find_last`, which returns either
-1 — rejected before the call — or an index already bounded by `zip_count(z)`,
the same field `_zip_ent` bounds against, so `zip_entry_size` cannot answer -1
there. The guard is defensive. The mechanism was also mis-stated: `alloc(0)`
returns 0, so an unguarded -1 would store at address -1, not one byte before a
buffer. The out-of-memory detail no longer renders through
`_agnosai_pkg_zip_detail` either — that printed "sankoch error 12", which is
`ERR_UNSAFE_PATH`, the code this module emits for a real zip-slip refusal.

### Fixed — the CI per-suite bisect reported every failure as "exit 0"

`code=$?` inside `if ! cmd; then` is always 0 — `!` inverts the status and the
branch runs when the inversion is 0, so `$?` reported the inversion rather than
the program. That discarded the distinction between 139 (SIGSEGV), 124 (the
`timeout`) and 1 (a failed assertion), which are three different
investigations. `|| code=$?` reads the real status and signals are now named.

## [2.0.0] — 2026-08-14

### Changed — rate limiting is mounted by default ([ADR 021](docs/adr/021-rate-limit-mounted-by-default.md))

**A deliberate wire divergence from the oracle**, which ports
`rate_limit_middleware` and never installs it. `agnosai_serve` now mounts a
limiter at **100 req/s per client key, burst 200**, tunable with
`AGNOSAI_RATE_LIMIT` and `AGNOSAI_RATE_LIMIT_BURST`; **`AGNOSAI_RATE_LIMIT=0`
disables it and restores the oracle's exact wire**. The reasoning, the numbers,
and what per-key limiting does *not* defend against are in the ADR.

⚠ **Two defects had to be fixed first — mounting on top of either would have
been worse than not mounting at all.** Both are below.

### Fixed — the rate limiter refused nothing at all

`agnosai_rate_limit_check` passed its `key` to majra's `ratelimit_check` as a
`Str`. majra keys its bucket map on a **cstr**: `KeyTypeCstr` hashes with
`hash_str` and compares with `streq`, both reading bytes straight from the
pointer. Handed a `Str` VALUE, both read the Str **header** instead — whose
first eight bytes are the data pointer — so identical content at two addresses
hashed two different ways.

Since the key is derived per request, **every request minted its own bucket with
a full burst and no request was ever refused.** Measured through the handler:
three identical requests produced **3 active keys and 0 rejections**; after the
fix, 1 key and the third request 429s.

The key is now converted with `_agnosai_rl_cstr_a`, built through the **request
arena** rather than the stdlib's `str_cstr`, which allocates on the no-free
global bump and would leak on every request. Safe because **majra 2.6.4 owns its
copy of the key** — the other half of this fix, released upstream.

Proven rather than asserted: two `Str`s with identical content at different
addresses hash to `-5808573365828332785` and `-5808599754107409849` under
`hash_str`, to the same `790097504932053221` over their actual bytes, and
`streq` on the `Str` values returns `0`.

### Fixed — every client shared one rate-limit bucket

`agnosai_serve_handler` called
`agnosai_rate_limit_client_key(str_from(""), 0, str_from(""), 0)` — both
"header present" flags hardcoded to `0` — so the key was **always the
`"unknown"` fallback** whatever headers arrived. The oracle reaches that value
only when neither header is present; the port made it the only outcome, so one
noisy client could 429 everyone, `/health` included. The handler now reads
`X-Forwarded-For` and `X-Real-IP` through `sandhi_server_find_header{,_a}` and
passes them, matching the oracle's `extract_client_key`.

⚠ **A helper-level test cannot catch this** — an earlier suite asserted
`agnosai_rate_limit_client_key` and bucket independence directly, and a mutation
reverting the handler passed the whole suite. The regression drives
`agnosai_serve_handler` over a pipe with two different `X-Forwarded-For` values,
which is the real path.

### Fixed — the mounted-by-default limiter had no bound on memory

Found by an adversarial review of the mount itself, and every one of these was
inert while the middleware was unmounted.

- ⚠ **Nothing ever swept.** `agnosai_rate_limit_evict_stale` had exactly one
  caller in the tree and it was a test, so every distinct key a client presented
  was retained for the life of the process — majra keeps a permanent copy of the
  key plus a bucket plus a map slot, and the key comes from an unauthenticated,
  client-settable header. A sweep now runs amortised, one walk per 256 checks.
  The sweep counter is an `atomic_fetch_add` against a modulus, not a
  `load`/`store` pair: every request thread runs it on the shared limiter, so a
  plain read-modify-write loses counts and delays the sweep by an unbounded
  amount exactly when load is highest. ⚠ **Not mutation-covered** — the suite is
  single-threaded, where the racy and atomic forms are indistinguishable, so
  reverting it passes all 260 assertions. Recorded as reasoning, not as a
  verified claim.
- ⚠ **Idle eviction alone does not bound cardinality.** A client presenting a
  fresh key on every request keeps every bucket well inside the 300 s window, so
  the sweep dutifully finds nothing while the map grows without limit. Above
  `AGNOSAI_RL_MAX_KEYS` (4096) the sweep now escalates to a 1 s threshold and
  then to 0.
- ⚠ **The key had no length cap.** `X-Forwarded-For` is copied whole and the
  request ceiling is 10 MiB, so one request could hand majra a megabyte-long key
  it then copies permanently — and anything over 4 KiB takes the freelist's
  large path and mmaps a VMA that is never unmapped, an unauthenticated route to
  exhausting `vm.max_map_count`. Keys are now truncated to
  `AGNOSAI_RL_MAX_KEY_BYTES` (64), enough for a textual IPv6 address with a zone.
- **`agnosai_rate_limit_client_key` leaked on every request.** `str_sub` and
  `str_from` are `default_alloc()` wrappers — the global **no-free** bump — so
  each call stranded 16 to 32 bytes permanently, and a *refused* request builds
  its key before the 429, so the attacker set the leak rate rather than the
  configured limit. Added `agnosai_rate_limit_client_key_a` and the `_a` helper
  variants beneath it; the handler passes its request arena.

### Fixed — `/health`, `/ready` and `/metrics` are exempt from the limiter

⚠ **Mounting by default without this is a self-inflicted outage.** A kubelet
sends neither proxy header, so its probe keys as the shared `"unknown"` bucket;
an unauthenticated flood of `GET /health` drains that bucket, the liveness probe
starts taking 429, and the orchestrator kills the pod — a crash loop manufactured
by the control meant to prevent one. The exemption is resolved through the router
rather than by comparing path literals, so it cannot drift from
`agnosai_route_needs_auth`.

### Fixed — `AGNOSAI_RATE_LIMIT` was parsed by the u16 **port** parser

`agnosai_serve_parse_port` returns its -1 sentinel above 65535, so an operator
setting `AGNOSAI_RATE_LIMIT=200000` — asking for effectively no limit — silently
got the 100 req/s default and 429s on almost all legitimate traffic, `/health`
included, with nothing logged. Same for `AGNOSAI_RATE_LIMIT_BURST`. Replaced with
a strict parser that rejects empty, non-digit and overflowing input.

⚠ **`str_to_int` would have been worse, not better**: it skips non-digits rather
than rejecting them and answers **0** on garbage — and `0` is the value that
*disables* rate limiting, so a typo would have quietly turned the control off.

The rate-limit defaults and their env parsing moved from `src/main.cyr` to
`src/server/rate_limit.cyr` (`agnosai_rate_limit_env_x1000` /
`agnosai_rate_limit_env_burst`), where they belong and where a test can reach
them — `src/main.cyr` cannot be included by a `.tcyr`.

### Fixed — the `AGNOSAI_RATE_LIMIT=0` audit line was truncated

`sakshi_warn(..., 44)` against a 46-byte message, so the single log record that a
security control had been switched off read `...by AGNOSAI_RATE_LIMIT` — naming
the variable but dropping the `=0` that identifies the setting responsible.

### Fixed — a function-level parity audit of every module against `rust-old/`

An eight-group sweep comparing each oracle symbol to its port raised 30 candidate
remainders; 18 were refuted on inspection and **20 of the surviving 22 are fixed
here**, each with a mutation that reproduces the original defect. The two left
open are named at the end.

**The limiter, the callback and the deadline — behaviour that was simply absent:**

- **`POST` to an A2A callback URL sent NO BODY.** `agnosai_guarded_fetch`
  hardcoded sandhi's body arguments to `0, 0`, so `AGN_CB_BODY` was written and
  never read: the delegating system was woken by an empty POST and had to
  re-poll for the result the callback exists to deliver. It also carried the
  *request* where the oracle sends the *response* (`a2a.rs:157`). Added
  `agnosai_guarded_fetch_body`; the payload is now rendered at dispatch time,
  because the response value tree lives in the request arena and that arena is
  reset before a detached thread could read it.
  ⚠ **A seam was required to test it at all** — the SSRF gate refuses loopback,
  so no local listener can observe the wire, and reverting the body to `0, 0`
  passed the entire suite. `_agnosai_serve_callback_fp` is the same injectable
  transport `agnosai_fetch_package_via` already uses.
- **A Parallel crew never observed its deadline.** The cooperative check sat on
  the sequential and DAG loop heads only, so a crew past `budget.max_duration_secs`
  ran to completion and reported its results as if it had finished in time.
  ⚠ **Adding the check crashed the server before the test caught it**: the wave
  loop builds every job up front, so breaking early left a tail with
  `AGN_CJ_RESULT == 0` and the collection loop dereferenced it. Both halves are
  mutation-covered, one of them by the segfault.
- **`CrewProfile.sandbox_strength` was computed nowhere.** The field, its setter
  and its serialization were all ported and covered by `tests/core_crew.tcyr`,
  and nothing outside a test ever called the setter — so every profile omitted a
  number the oracle emits. Now scored from `SandboxPolicy::process()` (60).

**Silent wrong answers:**

- **An uppercase UUID 404'd a crew that exists.** Four wire sites validated with
  `agnosai_uuid_is_valid` (which accepts uppercase) and then used the RAW segment
  as a `map_get` key on a Str-keyed map, where ids are minted lowercase. On
  `submit_approval` that meant **200 with `delivered: false`** on a
  human-in-the-loop gate. `_agnosai_uuid_key_a` gives `Path<Uuid>` semantics —
  parse, then look up by value — and allocates nothing for an already-lowercase
  key.
- **`redact` stopped after 1000 replacements** of a prefix, leaving occurrence
  1001 onward verbatim with nothing logged; and it bounded the token with four
  ASCII bytes where the oracle uses `char::is_whitespace`, so a key followed by a
  form feed or NBSP made it swallow the rest of the response.
- **`tool_input_get_int` passed negatives through.** The oracle's `as_u64` is
  unsigned, so any negative is `None`; `-1` only looked right because it collides
  with `AGNOSAI_NO_LIMIT`. It reached the wire as `"max_tokens": -5`.
- **`from_value` was lenient where serde is strict** — `Task.description`,
  `TaskResult.{output,status}` and three `CrewProfile` fields carry no
  `#[serde(default)]`. Defaulting `status` to Pending was the worst of them: a
  completed result came back reporting itself unfinished.
- **The relay handed majra `str_data` where a cstr is required.** `str_cat`,
  `str_sub` and `str_new` do not append a terminator, so a node id, target or
  topic built rather than written as a literal ran off the end of its buffer.
  Same class as the majra 2.6.4 ratelimit key bug; literals happen to be
  terminated, which is why every test passed.
- **`f64_max` propagated NaN** where Rust's documented contract is "ignoring
  NaN", poisoning `max_next_q` and every Q value derived from it.
- **The interner handed out `u32::MAX`** — the oracle's `checked_add` panics
  before the mint returns, so 4294967294 is the largest usable id.
- **`load_all_from_dir` chose the error by extension before reading**, so
  `/nonexistent.txt` gave `InvalidDefinition` where the oracle gives `Io`. The
  port argued this was better and cheaper; it is a divergence without an ADR, and
  conforming is cheaper than justifying a saved `open` on an error path.
- **`concurrent_users: 0` ran a real one-worker load test.** The oracle applies
  only `.min(500)`; zero is legal and answers "no requests completed". The port
  raised it to 1 and sent traffic the caller asked it not to send.

**Records that were never written:** the A2A over-length and metadata-limit
rejections, the crew-execution error, the `error` key on crew validation warnings
(taken as a parameter and never emitted), the WASM package-scan completion count,
and `endpoint`/`service_name` on the OTLP init line.

**Tests for behaviour that had none:**

- **A concurrent `ToolRegistry` test.** The oracle got no-lost-updates from
  `DashMap`; the port replaced it with a mutex the module header calls mandatory,
  and nothing drove the registry from two threads. ⚠ **The oracle's own shape
  cannot detect a lost update** — 20 writers over two names converge whether or
  not the map is guarded, and commenting out every `mutex_lock` left it green.
  Rewritten with disjoint key ranges: the mutation now yields **850, 783 and 969
  of 1000** across three runs.
- **WASM stdout capture and stdin delivery**, the module's core contract per
  ADR 019 and the one claim its tests did not check — `config_stdin` could have
  been dropped entirely with the suite green. Both now run through a hand-encoded
  184-byte WASI module that echoes stdin to stdout.
- ⚠ **The NaN test needed its NaN LAST.** `f64_max(a, b)` returns `b` whenever
  `f64_gt(a, b)` is false, and every comparison against NaN is false — so with
  the NaN first the next finite value overwrites it and the bug hides.

### Performance — B2's tail, and why it was smaller than advertised

`crew_runner` was carried as "63 bayan calls, the largest un-threaded allocator
surface left". Classified by **lifetime** rather than counted: 16 build event
payloads a subscriber reads after the emit, 14 build task-result metadata, 6
build the profile — all retained, none movable. **Exactly 4 are transient.** That
site is now pooled: **2,504 -> 2,280 B per `build_request` with three context
keys (-8.9%)**, no-context path unchanged at 1,744 B.

⚠ The first measurement of this showed no change and was meaningless: with no LLM
client `agnosai_execute_task` takes the placeholder arm and never calls
`build_request`.

### Fixed — `lib/` had been stale against the pinned snapshot

⚠ **agnosai was building against sandhi 1.9.9 all session.** `lib/sandhi.cyr`
still carried 1.9.9 while the pinned 6.5.20 snapshot had been refreshed to
**1.9.10** — the null-`body_ptr` fix released earlier the same day and folded
upstream. `cyrius lib sync --full` had never been re-run after that fold, and
`check-clean`'s lib-snapshot gate only compared again once a dep pin changed.

It masked nothing (both call sites guard the null independently), but every
suite result and allocation measurement taken before this ran against the old
sandhi. **The three-step is `lib sync --full` -> `deps` -> `build`, and verifying
`lib/` AFTER a build is the only reading that counts** — bumping a dep pin and
running `deps` alone does not refresh the snapshot.

### Fixed — five defects a second adversarial review found in the FIRST round's fixes

⚠ **Three of these were introduced while fixing something else**, which is the
part worth carrying forward: a fix is a change, and a change earns the same
review as the code it replaces.

- **CRITICAL — `concurrent_users: 0` killed the whole server.** Removing the
  lower clamp to match the oracle's bare `.min(500)` was only half the change:
  **the oracle's floor lives at the DIVIDE**, `MAX_TOTAL_REQUESTS /
  concurrent_users.max(1)` (`load_testing.rs:135`). Without it,
  `100000 / 0` reaches an i64 `idiv` and raises **SIGFPE**, which is not
  per-thread recoverable — an unauthenticated `POST /mcp` with one field takes
  down all 100 workers. Mutation exits **136**. The clamp assertions could never
  have caught it; the test now drives `agnosai_load_test_run` with 0.
- **CRITICAL — the crew-context arena could wipe the process heap.**
  `_agnosai_crew_ctx_arena` fell back to `default_alloc()` when the pool could
  not hand one out, and the caller then passed that to
  `agnosai_arena_pool_release` — whose `reset_via` is `_bump_reset` ->
  `alloc_reset()`, which **zeroes and rewinds the entire global heap** while
  every other pooled worker still holds pointers into it. Degradation under
  memory pressure became whole-process memory corruption. Acquire failure now
  means "no arena": the tree falls back to the global bump and is never
  released. ⚠ **Not test-covered** — with the pool built eagerly, acquire
  returns 0 only on mmap failure, so the path is OOM-only. The fix is
  structural, and that is recorded as reasoning rather than as a verified claim.
- **HIGH — the arena pool was built lazily from worker threads.**
  `src/arena_pool.cyr`'s own header says: *"Build the pool EAGERLY on the main
  thread. A lazy `if (pool == 0)` double constructs when two spawned threads
  race it."* This code did exactly that, and
  `_agnosai_crew_run_parallel` spawns a worker per task, so the race is a crew's
  first wave rather than a corner. Now a module-scope initializer, matching
  `_AGNOSAI_AUDIT_POOL`.
- **MEDIUM — the security-audit error kind was a process global.**
  `_agnosai_audit_last_err` was written and read across two HTTP chains with a
  15 s budget each, from `run_pooled` worker threads — a seconds-wide race in
  which one audit zeroes another's kind (reverting to the generic message) or
  supplies its own, reporting "TLS handshake failed" for a connect failure and
  leaking one request's outcome into another's response. Replaced with
  `agnosai_run_security_audit_err(url, out_kind)` and a stack local.

### Fixed — a flaky test that only failed in the full sweep

The A2A callback test waited on the detached thread with a fixed spin count
(`spins < 20000`). That passed whenever the suite ran alone and **failed in the
99-suite sweep**, reading a body the thread had not written yet — a busy spin
also starves the thread it is waiting for. Now a `sleep_ms` loop with a
two-second wall-clock bound, asserting the bound was not hit so a genuine hang
still fails. ⚠ 8 synthetic spinners did not reproduce it; the fix stands on
iterations not being a duration, not on a repro.

### Changed

- **`[deps.kavach]` 3.11.11 -> 3.11.12** — closes the *memory* half of ADR 019's
  residual, the last thing that ADR carried as open. This port was already
  bounded (`agnosai_wasm_execute` has sent a memory policy since 2026-08-11),
  but a kavach caller on `policy_basic()` got **no ceiling at all** — verified
  against wasmtime 47, where a module declaring 128 MiB instantiates freely and
  is refused under 64 MiB. kavach now always emits one, defaulting to the same
  64 MiB `DEFAULT_MAX_MEMORY_BYTES` the oracle's `WasmSandbox` uses.
- **`[deps.majra]` 2.6.5 -> 2.6.6** — the same review found that honouring the
  relay capacity (2.6.5, filed from here) turned a latent divergence into a live
  deadlock: both fan-out paths pushed with the blocking `chan_send`, where
  Rust's `broadcast::send` never blocks. A depth-2 relay wedged its sender on
  the third send, permanently, since majra has no unsubscribe. ⚠ The 2.6.5 test
  missed it because it filled the ring with `chan_try_send` **directly on the
  channel**, routing around the blocking call.

### Fixed — two error paths that could only say "it failed"

Both needed the reason plumbed out of an API that discarded it, which is why
they were held back from the first pass rather than patched as text.

- **The A2A error field never said anything.** The oracle sends the
  orchestrator's own error to the delegating system (`a2a.rs:182` is
  `error: Some(e.to_string())`); the port answered a fixed "crew execution
  failed" — which is the literal `crews.rs:236` uses for a *different* route.
  ⚠ **The reason already existed and was thrown away**: `_agnosai_orch_finish`
  builds a `run_err` for `agnosai_crew_runner_run` — the cyclic-DAG arm the
  oracle's `?` propagates — and then returned a bare 0. New
  `agnosai_orchestrator_run_crew_err`; the bare form delegates with `out_err`
  of 0, which means "don't care".
- **Every security-audit transport failure read "request failed".** The oracle
  reports the underlying error (`security_audit.rs:132-136` maps reqwest's with
  `.map_err(|e| e.to_string())`), so an operator sees DNS, TLS, connect or
  timeout. `agnosai_guarded_fetch` computed `sandhi_http_err_kind` and dropped
  it; `agnosai_guarded_fetch_full` surfaces it and the tool names the cause.

  ⚠ **The first test for this was vacuous** — it asserted the reason TABLE,
  which is pure, so dropping the `store64` that fills it left all six assertions
  green. The test now drives the real path against an RFC 2606 `.invalid` host,
  which fails with `SANDHI_ERR_DISCOVERY` (kind 8). Same shape as the
  `_agnosai_rl_parse_count` lesson: **testing a helper does not test the
  wiring.**

- ~~The WASM fuel budget~~ ✅ **closed** by kavach 3.11.11 — see the dep bump
  above. ADR 019's Residual no longer describes an open gap.

### Changed — toolchain pinned to cyrius 6.5.21

Was 6.5.20. `lib/` re-synced from the 6.5.21 snapshot (107 files) and verified
after a build; `cyrius --version` and the manifest pin agree, so the drift the
wrapper had been reporting is gone.

### Performance — the limiter's cost on the default request path, measured

⚠ **`benches/server.bcyr` included `src/server/rate_limit.cyr` and benchmarked
nothing from it**, so mounting the limiter by default put a cost on every
request that no row covered — and a bench sweep showing "no regressions" was
never evidence about this change. Four rows added:

| row | measured |
|---|---|
| `rate_limit_check_hot` | **1.511 µs** (min 1.508, max 1.517) — steady state, bucket already minted |
| `rate_limit_check_new_key` | **2.189 µs** — first request from a key: mints a bucket and majra's permanent key copy |
| `rate_limit_client_key_global` | **190 ns** — key derivation on the global bump |
| `rate_limit_client_key_arena` | **174 ns** — the same through the request arena |
| `rate_limit_sweep_1k_live` | **7.227 µs** over 1,000 live keys with nothing stale |

Two things worth reading off these:

- ⚠ **The check is dominated by a single clock read, not by the bucket.**
  `time_now_ns` is unavoidable for a lazy-refill token bucket, and this box
  measures a clock read at **1.211 µs** (`clock_epoch_secs_baseline` = 1.210 µs,
  and the harness reports the same figure as its own timer floor). So ~80% of
  `rate_limit_check_hot` is the clock; the mutex, map lookup, arithmetic and
  counters together are the remaining ~300 ns. Optimising the bucket would move
  almost nothing.
- **The arena fix is faster as well as leak-free** — 174 ns against 190 ns, so
  routing the key through the request arena costs nothing and saves the 16-to-32
  bytes per request the global-bump form stranded permanently.
- **The sweep amortises to ~28 ns per check** (7.227 µs / `AGNOSAI_RL_SWEEP_EVERY`
  = 256), and roughly 113 ns per check at the `AGNOSAI_RL_MAX_KEYS` ceiling.

### Changed

- **`[deps.kavach]` 3.11.10 -> 3.11.11** — adds `config_fuel`, so
  `agnosai_wasm_sandbox_fuel` finally advertises a limit the runtime enforces.
  kavach derived its own budget from the timeout (`timeout_ms * 1e6` = **3e10 at
  the default 30 s, 30x** the oracle's `store.set_fuel(1e9)`), and the two
  cannot be conflated without capping wall time at one second. Filed from here;
  `agnosai_wasm_execute` now sends the configured budget, and
  `agnosai_wasm_sandbox_set_fuel` is the port-local setter the oracle does not
  need (it builds the struct directly).
- **`[deps.majra]` 2.6.4 -> 2.6.5** — closes both relay divergences this port
  had been carrying as "owed to majra": `capacity` was accepted and discarded
  (`relay_subscribe` hardcoded `chan_new(256)`), and the message timestamp was
  `CLOCK_MONOTONIC` where the oracle stamps `DateTime<Utc>` — meaningless once
  the message leaves the emitting process. `tests/fleet_relay.tcyr` had a block
  deliberately asserting the *broken* behaviour so a reader saw it as known
  rather than accidental; those two assertions now assert the oracle's.
- **`[deps.majra]` 2.6.3 -> 2.6.4** — carries the limiter's half of the key fix:
  it now owns its copy of the bucket key instead of borrowing the caller's,
  eviction returns both key and bucket to the freelist instead of leaking them,
  and the sweep itself no longer leaks its own scratch vecs on the global bump —
  which matters now that agnosai calls it on a timer rather than never.

### Added

- Four decisions closed, all five now settled: **D1** (mount `rate_limit`) —
  **yes**, per ADR 021; **D3** (memoize `builtin_presets()`) — **no**, the route
  is cold and the oracle's shape stands, so no ADR is owed.

### Added — `src/arena_pool.cyr`, a bounded pool of reusable arenas

Port-local; no oracle counterpart is possible, since Rust frees on drop and
`rust-old/` never had to think about arena ownership.

**The defect.** `arena_allocator(n)` is `alloc(56)` + `alloc(n + 16)` +
`allocator_new`'s `alloc(40)`, all on the **no-free global bump**, and
`reset_via` -> `arena_reset` only rewinds three pointers — the chunk is never
returned and the next call builds a new one. Three sites did that per operation:

| site | was | now |
|---|---|---|
| `tools/agnos.cyr` — **every outbound HTTP request** | **5,242,992 B** | **0** |
| `tools/remote_registry.cyr` — per package fetch | 12 MiB + 112 B | pooled, 2 slots |
| `tools/builtin/security_audit.cyr` — per audit | 2.5 MiB + 112 B | pooled, 4 slots |

Measured on the agnos path: **0 bytes per exchange** against **5,242,992**
before, over 32 cycles, with 8 arenas minted and **0 fallbacks**.

**The design**, and why not the alternatives — an escape analysis over all five
per-operation sites found **nothing escapes any of them** (each already
deep-copies before its `reset_via`), so this is an ownership problem, not a
lifetime one, and the lifetime fixes do not apply:

- **A thread-local arena is unavailable**: `thread_local_get` faults on the main
  thread and `thread_local_init` on a worker orphans the block `thread_create`
  installed, taking sandhi's request-arena slot with it (`audit.cyr:386`).
- **sandhi's request arena is worse, not better.** It is 64 KiB with
  `ARENA_FULL_SPILL`, and sandhi allocates its receive buffer eagerly at the full
  cap, so pointing agnos at it converts 5 MiB of untouched VA into ~4 MiB of
  RSS-charged **permanent** bump per call. It is also reset by sandhi between
  requests, which would rewind live state at the sites that reset mid-operation.
- **One shared arena under a mutex** would serialize outbound HTTP across all
  pool workers.

So: a `chan_new(slots)` prefilled with growable arenas; acquire is
`chan_try_recv`, release is `reset_via` + `chan_try_send`. The channel is already
mutex-guarded, so the pool adds no synchronisation of its own.

⚠ **Acquire never blocks** — an empty pool mints a fallback. `chan_recv` blocks
and there is no `mutex_trylock`, so blocking would hang a sandhi worker, and
`load_testing` holds *two* arenas per worker, which is a two-resource deadlock.

⚠ **No ownership marker.** Release just tries to send; a full channel means the
arena is surplus and is dropped. Pooled and fallback arenas are interchangeable,
so the channel's capacity is what bounds the pool.

⚠ **Arenas are growable, not fixed** — they converge on each site's true
high-water mark and then allocate nothing. A fixed arena must either waste its
ceiling or return 0 on an oversized request, and the latter is exactly where the
null-body class of defect came from.

⚠ **`security_audit` resets its arena MID-operation**, between the GET and the
CORS probe. Those stay plain `reset_via`; only terminal ones release to the pool.

⚠ **Pools are built eagerly at module scope.** A lazy `if (pool == 0)` double
constructs when two worker threads race it, and the loser's arenas leak.

`tests/arena_pool.tcyr` — **33 assertions**, four mutations all killed: acquire
that never reuses, release that drops instead of parking, release that skips the
reset, and an exhausted pool that returns 0 instead of falling back. The
load-bearing one is `alloc_used()` delta **== 0** over 64 cycles; a pool that
quietly minted per acquire would pass every functional assertion and fix nothing.

⚠ An earlier draft asserted arena **identity** after release on a 4-slot pool and
failed — the channel is **FIFO, not LIFO**, so a released arena goes to the tail.
Identity is now checked against a one-slot pool, where the orderings coincide.

**All five sites are pooled** as of the follow-up below.

### Added — the last two arena sites are pooled

⚠ **My earlier note that these two "cannot reach a request-path pool" was wrong,
and worth correcting rather than quietly deleting.** The pool is a process-global
whose channel is mutex-guarded — nothing about it is request-scoped, so a
detached callback thread or a load-test worker can acquire and release exactly
like a sandhi worker. The caution cost a round trip and produced nothing.

| site | was | now |
|---|---|---|
| `server/serve.cyr` — per a2a callback | 64 KiB + 112 B | pooled, 4 slots |
| `tools/builtin/load_testing.cyr` — **twice PER USER** | `max_requests*40 + 65536` and 1 MiB | pooled, two pools, 16 slots each |

`load_testing` was the only site whose cost scaled with a caller-supplied number:
`users` workers x two arenas, none returned. It gets **two** pools because the
two arenas have different roles, and the persistent one's size varies with the
user count (`max_requests = 100000 / users`, so ~4 MiB at one user and ~73 KiB at
500). A pool cannot pick one number for that — and does not need to, because
pooled arenas are **growable** and converge on what each run actually uses. The
now-dead size computation is removed; the constants stay as documentation of
where that convergence lands.

⚠ **Two resources per holder cannot deadlock here** because acquire never blocks:
a worker that finds either pool empty mints a fallback instead of waiting. That
is the property the non-blocking design was chosen for.

⚠ `load_testing`'s per-request scratch reset (`:251`) stays a plain `reset_via` —
only the terminal releases after aggregation return arenas to the pools.

### Fixed — the a2a callback thread leaked its entire stack mapping

`agnosai_serve_dispatch_callback` called `thread_create` and **discarded the
handle**. `thread_join` is the only caller of `munmap_stack`, so nothing ever
released the mapping. `mmap_stack` maps `THREAD_STACK_SIZE + THREAD_GUARD_SIZE`
= **2,101,248 bytes** and mprotects the guard page, splitting it into **2 VMAs**
— per callback, permanently, plus a 4 KiB TLS block and a 24-byte handle on the
no-free bump that `thread_join` would not have reclaimed either.

Measured on this tree before the fix: **20 unjoined threads left +40 VMAs** that
persisted after the threads had exited; **20 detached left +0**.

Now `thread_create_detached`, which has the child unmap its own stack in the
trampoline tail and carves the TLS from the top of that same mapping so one
`munmap` frees both. It also **reports failure** instead of returning 1
unconditionally for a dispatch that never started.

⚠ **The identical fix already shipped in `orchestrator/submit_crew`** — see its
header and the upstream filing that produced `thread_create_detached` in cyrius
6.5.8. This site was simply missed. Audited the rest: `crew_runner:1066` and
`load_testing:471` keep their handles and join; `inference_queue:437` is already
detached; `agnosai_serve_install_signals` spawns one thread for process lifetime,
not per operation.

⚠ **The call site is not offline-reachable, so no assertion covers it** — the
gate refuses loopback and private ranges by design, and a public host means DNS
and a 30 s connect timeout that leaves the stack mapped while any assertion
looks. `tests/server_serve.tcyr` pins the *primitive* instead
(`_t_detached_threads_free_their_stacks`), and says so. An earlier version of
that test used a loopback URL and was **vacuous**: the gate refused it before any
thread spawned, so reverting the fix passed the whole suite. The replacement is
mutation-verified.

### Fixed — a null-pointer read on large AGNOS / remote-registry responses

sandhi could return **`SANDHI_OK` with a null body pointer** and a positive
`body_len`. `_sandhi_resp_frame_a` (`lib/sandhi.cyr:3345`) assigned
`_sandhi_resp_body_copy_a`'s result into `body_ptr` **without checking**, and
that copy answers 0 when the arena cannot fit it. Two agnosai call sites then
read from address 0:

- `src/tools/agnos.cyr` — `bayan_json_v_parse_buf(0, len)`;
  `bayan_json_v_parse_ctx_a` (`lib/bayan.cyr:3773`) has no null-buf guard.
- `src/tools/remote_registry.cyr` — `str_from_buf(0, len)` is `alloc` + `memcpy`
  (`lib/str.cyr:609`), copying **from** address 0.

⚠ **It needs no memory pressure to fire.** sandhi allocates its receive buffer
eagerly at the full `max_response_bytes` cap (`lib/sandhi.cyr:4644`) before a
byte arrives, so the slack left for the body copy is the arena minus that cap:
**~1 MiB** for agnos (4 MiB cap, 5 MiB arena) and **~2 MiB** for remote_registry
(10 MiB cap, 12 MiB arena). Any body above those took the null path. Neither
arena sets a spill policy, so `ARENA_FULL_NULL` makes the failed alloc a 0.

Both sites now check `sandhi_http_body(r) == 0` with a positive length and fail
closed — agnos reports `AGNOSAI_AGNOS_BAD_JSON` with the detail
`"response body could not be buffered"`, remote_registry a download failure.

⚠ **The guards stay even after the upstream fix.** agnosai must not depend on a
transport's internal invariant to avoid a segfault.

**Fixed at the root in sandhi 1.9.10**, released and tagged alongside this —
framing now returns `SANDHI_ERR_INTERNAL` rather than reporting OK, matching the
guards that function already applies to `outp_cell` / `outl_cell`. ⚠ **agnosai
does not have that fix yet**: `lib/sandhi.cyr` is `SANDHI_VERSION = "1.9.9"`
because sandhi is folded into the cyrius stdlib rather than taken as a git dep,
so it arrives in the **next cyrius release** — 6.5.20 predates it. Until then the
local guards are the whole protection, and they stay afterwards regardless.

⚠ **Not reachable from the shipped binary today** — and the sweep that found it
claimed otherwise, via `_agnosai_mcp_tools_call`. Checked:
`agnosai_register_{mneme,synapse,delta}_builtins` have **zero callers in `src/`**;
`main.cyr:351-353` registers only basic, load_testing and security_audit, so no
agnos-backed tool is ever in the registry. Latent, and live the moment one is.

### Removed

- **A redundant whole-package copy in `remote_registry`.** `str_from_buf` already
  allocates on the global bump and memcpies, so the `str_clone` that followed it
  duplicated the entire download — ~2N bumped for an N-byte package, the first
  N+16 orphaned immediately on a no-free allocator.

### Performance — three allocation defects found by the P(-1) sweep

- **`agnosai_prompt_scan_input`: 560 B per CLEAN scan → 0.** It ran 35 `str_from`
  calls on the clean path — 31 literal needles, plus four message `Str`s built
  eagerly at the top whether or not anything matched. The oracle allocates
  **none**: `rust-old/src/server/prompt_guard.rs:30` is a
  `const INJECTION_PATTERNS: &[(&str, &str)]` static table.

  Needles now go through `agnosai_str_contains_ci_cstr` (`src/strcase.cyr:96`,
  which exists for exactly this and whose comment says so), and a message is
  built only on the branch returning it — 16 B on a detection, 0 otherwise. This
  runs on every task description, expected output and context blob, so the old
  cost was per-request. Mutation-verified both ways.

- **`_agnosai_path_matches_at` captured on every pattern TRIED, not on the match.**
  It ran `str_sub_a` the moment it reached a `*` segment and only afterwards
  compared the remaining segments, so `/api/v1/crews/*` tried against
  `/api/v1/crews/abc/cancel` captured `"abc"` and then failed. The capture is now
  deferred to a confirmed full match: **64 B → 48 B per parameterised resolve**
  (−25%), which is exactly one `Str` header saved.

  ⚠ **`tests/server_router.tcyr:453` asserted a property the matcher violated**,
  under a `< 192` bound that stayed green up to eleven captures. It now asserts
  `deep hit + exactly one Str`, both measured in the same run. `str_sub_a`
  **borrows** rather than copying — verified, a 1-char and a 46-char parameter
  cost identically — so one capture is 16 B regardless of path length.

- ⚠ **`orchestrator/audit.cyr` was a FALSE POSITIVE** and needed no change. Its
  `_agnosai_audit_sign_a` is already fully threaded, and the two bare
  `str_builder_putc` calls are documented as safe at the site: 64 hex chars into
  the 64-byte inline buffer is an exact fit. **Measured 88 B/sign, entirely the
  deliberate `str_clone`** that exists because the signature outlives the scratch
  arena; `putc` contributes zero. The 5 bare adds + 1 bare build the sweep counted
  are all in `_agnosai_audit_failure`, an error path whose result must outlive any
  request arena and which `agnosai_audit_verify` — no `_a` form, no caller in
  `src/` — reaches only on a corrupted chain.

### Fixed — the crew execution timeout was computed and never applied

`agnosai_orchestrator_timeout_secs` resolved the budget's `max_duration_secs` or
the oracle's 3600 s default, and had **zero call sites in `src/`** — its only
mention was a comment in `main.cyr`. The oracle wraps the run in
`tokio::time::timeout` and, on expiry, logs `crew execution timed out` and
returns `CrewState { status: Failed, results: vec![], profile: None }`
(`rust-old/src/orchestrator/orchestrator.rs:197-214`).

`POST /api/v1/crews` runs the crew inline, so a wedged crew held one of the 100
pool workers **forever** — no `Failed` state, no sakshi event, no audit record.

- `_agnosai_orch_finish` now stamps a monotonic deadline before the run, and the
  runner checks it at the two loop heads that already check cancellation. On
  expiry the orchestrator substitutes the oracle's bare `Failed` state, so
  **partial results are DISCARDED** rather than read as a finished crew.
- ⚠ **The deadline is COOPERATIVE, and this is a real divergence.** The oracle's
  `tokio::time::timeout` drops the whole future and aborts work in flight;
  Cyrius cannot abort a thread mid-syscall, so the check cannot fire until the
  current task returns. **A single task that hangs forever still holds the
  worker** — `src/llm/hoosh.cyr` sets no per-request timeout, so a wedged gateway
  connection is exactly that case. Closing it needs a request timeout on the LLM
  transport and is its own bite. Documented at the call site, not glossed.
- ⚠ **`agnosai_orchestrator_timeout_secs` gained a NULL-budget guard**, and it is
  load-bearing: three suites construct `agnosai_orchestrator_new(0)`, which was
  harmless only because nothing called the accessor. Wiring it put an unguarded
  `load64(0 + …)` on every crew run. `main.cyr`'s claim that a 0 "would fault the
  moment a crew runs" was **false when written** — nothing dereferenced it — and
  is corrected in place.

### Added — `output_filter` is wired into task output ([ADR 020](docs/adr/020-output-filter-wired-into-task-output.md))

⚠ **A deliberate divergence, not a bug fix.** `src/server/output_filter.cyr` —
20 functions, its own passing assertions — had no caller. **Neither does the
oracle's**: `rust-old/src/server/mod.rs:11` is `pub mod output_filter;` and that
is the module's only mention in the entire Rust tree. The port was at parity.

What made it worth changing is the asymmetry: `prompt_guard`, the *inbound* half
of the same defence, **is** wired in the oracle
(`rust-old/src/orchestrator/crew_runner.rs:651`) and at four sites here. agnosai
sanitised everything going into a model and inspected nothing coming out.

- **Scanning is unconditional on model output** — every finding is logged at
  `SK_WARN` with task id, category and pattern.

  ⚠ **But NOT on the no-LLM placeholder path, and that split is measured.**
  `agnosai_output_scan` costs **16.7 µs** against a ~40 µs/task crew path;
  scanning the placeholder unconditionally measured **+90%** on
  `run_crew_10_tasks_sequential` (394 → 749 µs) and +43% at one task. The
  placeholder echoes the task description — request input `prompt_guard` has
  already sanitised — so there is no model output there to protect. It is
  filtered only when redaction is enabled.
- **Redaction is OFF by default**, behind `AGNOSAI_OUTPUT_REDACT=1`.
  `agnosai_output_redact` rewrites the response, so a task legitimately returning
  an email address or a key-shaped token would have its answer mangled with no
  way for the caller to tell. Silently corrupting correct output is the worse
  failure. The gate is a settable flag seeded from the environment at startup
  rather than a per-call `getenv`, because a test process has no `setenv` and an
  undrivable gate is how the module went un-called in the first place.
- **Measured cost: +3.4% at one task, +1.0% at ten** (`run_crew_1_task_sequential`
  94.5 → 97.8 µs, `run_crew_10_tasks_sequential` 394.4 → 398.4 µs,
  `run_crew_10_tasks_parallel_4` flat) — and most of that residual is the crew
  timeout landing in the same release, not the filter.
- ⚠ **The LLM arm of the wiring is not mutation-covered**: reverting
  `agnosai_execute_task` to the raw response passes the whole suite, because that
  arm needs a live gateway — the same limitation `_agnosai_otlp_post` carries.
  The placeholder arm is covered (unwiring it fails two assertions).

### Fixed — OTLP export was silently non-functional, and leaked every span it encoded

Three defects in `src/telemetry/otlp.cyr`, found by the 2026-08-12 P(-1) sweep.
⚠ **There is no oracle counterpart to any of this**: `rust-old/src/telemetry/mod.rs:50`
hands the whole transport to `hoosh::telemetry::init_otel`, and ADR 003 keeps hoosh
a remote seam rather than linking it — so this file is port-original and its bar is
the OTLP/HTTP spec, not a Rust line.

- **The exporter POSTed to the collector ROOT, so nothing was ever ingested.**
  `_agnosai_otlp_post` sent `str_cstr(endpoint)` raw. `agnosai_otlp_path` — which
  exists solely to append the spec's `/v1/traces` — **had zero production
  callers**, only five assertions in `tests/telemetry_otlp.tcyr`. The documented
  deployment (`OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318`, the sidecar
  form this module's own header describes) received `POST /` and 404'd.
  `_agnosai_otlp_post_url` now appends the path when the endpoint carries none
  and passes a path-bearing endpoint through untouched, so a
  `..._TRACES_ENDPOINT` is not doubled into `/v1/traces/v1/traces`.

- **`OTEL_EXPORTER_OTLP_HEADERS` was unread, so no hosted collector was
  reachable.** Honeycomb, Grafana Cloud and Datadog all authenticate through it.
  Now parsed per the spec (`key=value` pairs, comma-separated, whitespace
  trimmed). ⚠ An over-long key or value is **skipped, never truncated** — these
  carry API tokens, and a truncated token fails at the collector as a 401 that
  reads like a credential problem rather than a parser one.

- **Both 256 KiB ring arenas held only builder headers; every span fragment and
  batch document went to the no-free global bump.** A `str_builder` does not
  remember its allocator — the header is 24 bytes (`lib/str.cyr:428`) — so
  `str_builder_new_a(arena)` charged the arena 88 bytes and the module's **29
  bare `str_builder_add*` and 4 bare `str_builder_build`** calls all grew through
  `default_alloc()`. `lib/str.cyr:433` states the contract; this module never
  honoured it. The encoder, escaper, hex writer and batch builder are now
  threaded end to end.

  `str_builder_putc` and `str_builder_add_json_str` have **no `_a` form**
  upstream, so `_agnosai_otlp_putc_a` and `_agnosai_otlp_json_str_a` are
  port-local stand-ins. The escaper is byte-for-byte the stdlib's — the suites
  assert the wire, and all four telemetry suites passed unchanged.

### Performance

- **OTLP export: ~2,736 bytes per span on the no-free global bump → 0.** Measured
  across a full enqueue+drain cycle at 1, 10, 32, 64, 128 and **256** spans (the
  ring's whole capacity): **0 bytes** at every rate, against ~1,496 B/span
  before. The residual 16 B in an earlier reading was the probe's own `str_from`,
  not the module's.

  **Time cost of the threading, measured A/B on the same tree** (the new
  `benches/telemetry.bcyr`, 20,000 iterations each):

  | shape | before | after | |
  |---|---|---|---|
  | `ring_enqueue` + periodic drain — **the production path** | 16.805 µs | **15.712 µs** | **−6.5%** |
  | `agnosai_otlp_span_json` — global form, encoder tests only | 11.964 µs | 13.112 µs | +9.6% |

  The production path got **faster**: arena bump allocation beats the global
  allocator by more than the vtable indirection costs. The convenience wrapper
  got ~10% slower because it now dispatches through an allocator handle where it
  used to call `default_alloc()` directly — it is on no production path, and
  paying there to make the ring both leak-free and faster is the right trade.

  `AGNOSAI_OTLP_ARENA_BYTES` 256 KiB → **768 KiB**, sized from
  `AGNOSAI_OTLP_RING_MAX` rather than from final document size. ⚠ A span fragment
  of ~700 bytes costs **2,736 bytes of arena** — `str_builder` doubles from a
  64-byte inline buffer and an arena has no free, so every abandoned intermediate
  stays. At 256 KiB the arena held ~95 spans and then spilled, and
  `ARENA_FULL_SPILL` spills to the global bump — reintroducing the leak at
  exactly the load that matters. Measured: 180,928 bytes spilled in one cycle at
  128 spans on the old size. The cost is virtual, not resident (anonymous
  overcommitted mmap; only touched pages count).

### Added

- **`tests/telemetry_otlp.tcyr` gains an allocator assertion — it had none**,
  which is why the un-threaded builders shipped. 115 → **122 assertions**:
  zero-global-bytes over a 64-span cycle, the same over a **full ring** (so an
  undersized arena cannot silently reintroduce the spill), and four
  `_agnosai_otlp_post_url` cases.

  ⚠ **Mutation-verified, and the first version of the comment overclaimed.**
  Four mutations killed it — `str_builder_build_a` → `build` on either arena,
  the escaper reverted to the stdlib's, and `str_from_a` → `str_from`. One
  **survived**: reverting a single short `str_builder_add_cstr_a`, because
  `_sb_grow` only allocates when capacity is exceeded, so an append into an
  already-grown buffer never reaches the allocator. The test says so rather than
  claiming to catch everything.

### Changed

- **Toolchain pinned to cyrius 6.5.20** (was 6.5.19). Full three-step, `lib/`
  diffed against the 6.5.20 snapshot **after the sync and again after a build**:
  zero drift, `cyrius deps --verify` **113 verified / 0 failed**, duplicate-fn
  warning count unchanged at **35**. Gate on the bumped tree: **97 suites**,
  coverage **1561/1561 (100%)**, `check-clean.sh` green (fmt 219, lint 122,
  doc 112, doctest 1).

  ⚠ **6.5.20's stdlib snapshot is byte-identical to 6.5.19's** — `diff -rq`
  across both `lib/` trees reports **0 differing files**. This is a pure
  compiler release, so the bump buys code generation, not modules. What it
  fixes:

  - **P1 — a `switch` / `match` case body could only be left safely by
    `return`.** A body that fell through after storing to a local produced a
    **wrong answer with no diagnostic**, or a segfault; broken on every target.
    Cause was the v5.6.27 regalloc NOP-harvest compactor, which repairs jump
    disp32s and fixup CPs but never knew the switch jump table existed —
    entries came out `16, 30, 47, 64` where correct is `16, 26, 39, 52`, off by
    exactly +4 per preceding case body. `case 0` alone worked, and bodies of
    `{ return N; }` never saw it: no local store, no NOP, no shift.

    **agnosai's own source is not exposed** — zero statement-position
    `switch` / `match` across `src/`, `tests/` and `benches/`. Three sites in
    the *vendored* surface carry the trigger shape: `lib/sakshi.cyr:1101`
    (`_sk_level_str`, ten arms, every one a `store64` pair with fallthrough —
    it formats every sakshi log line), `lib/kavach.cyr:795` and
    `lib/sigil.cyr:221` (both `syserr_print`, arms declaring locals, no `_ =>`
    default). ⚠ **Whether any of the three actually miscompiled under 6.5.19 is
    undetermined** — a minimal repro compiles correctly on 6.5.18, .19 and .20
    alike, because the defect needs enough register pressure for the picker to
    leave harvestable NOPs. The 97-suite baseline was green on 6.5.19, which is
    evidence against a live functional break but not proof, since the failure
    mode is a silent wrong answer.
  - `#derive` no longer inflates line numbering, removing one file-map entry
    per derive against the 1024 cap. agnosai authors **zero** `#derive`
    directives, but 93 real ones expand upstream of it across `lib/kavach.cyr`
    (35), `lib/libro.cyr` (27), `lib/ai-hwaccel.cyr` (16) and `lib/sigil.cyr`
    (15).

- **`[deps.bote]` → 3.3.1, `[deps.majra]` → 2.6.3**, both released as part of
  this chain rather than consumed from an existing tag.

### Fixed

- **The four-level dependency defect is CLOSED at every root, not absorbed by a
  counter-pin.** agnosai's own `[deps.sakshi]` was already gone; what landed now
  is the rest of the chain:

  ```
  agnosai -> bote -> [deps.libro] -> [deps.patra] -> [deps.sakshi] 2.4.2
              ✅ 3.3.1    ✅ 2.8.5      ✅ 1.13.0        ✅ gone
  ```

  In order, each verified before the next: **patra 1.13.0** (zero `[deps.*]`),
  **cyrius re-folds it**, **libro 2.8.5** (`[deps.patra]` 1.12.12 → 1.13.0),
  **bote 3.3.1** (defensive `[deps.sakshi]` 2.4.10 deleted). Re-confirmed here
  **after a build**: `lib/sakshi.cyr` holds at **2.4.10**, `lib/patra.cyr` at
  1.13.0, `lib/sigil.cyr` at 3.12.7.

- **A second stale-sakshi path, never previously identified.** libro fed the
  overlay through **two** pins, not one: **sigil 3.12.1's own manifest declared
  `[deps.sakshi]` at 2.4.3**, alongside patra 1.12.12's 2.4.2. libro 2.8.5 moves
  sigil to 3.12.7 (zero `[deps.*]`), which also closes the **3.12.5 PKCS#1 v1.5**
  and **3.12.6 RSA-PSS** authentication bypasses libro had been pinned behind.

### Security

- **RS256 JWT verification was an authentication bypass under load. Fixed by
  serialising the sigil call; the real fix belongs upstream.**

  > ⚠ **SUPERSEDED — read this box before acting on the entry below.** This is
  > the *original discovery* entry, and `[Unreleased]` is ordered newest-first,
  > so everything correcting it sits **lower in this same section**. Two of its
  > instructions are now wrong:
  >
  > - **`_agnosai_auth_rsa_verify_locked` NO LONGER EXISTS.** The mutex was
  >   deleted under the cyrius 6.5.14 / sigil 3.12.6 bump, after staging the
  >   removal across four sigil releases on the pinned-lane harness (3.12.2:
  >   888 forged accepted of 400,000; 3.12.5: 2000/2000 valid, 0 forged).
  >   `_agnosai_auth_validate_jwt` calls `rsa_pkcs1v15_verify_sha256` directly.
  >   "Do not remove that lock" is therefore an instruction about code that is
  >   gone — do not re-add it on the strength of this paragraph.
  > - **The upstream fix landed.** Re-verified against the cyrius 6.5.19 fold
  >   (sigil 3.12.7): every buffer on the RS256 verify path is function-scope,
  >   hence per-call and per-thread — `rsa_pkcs1v15_verify_sha256` (hbuf/dbuf),
  >   `_rsa_pkcs1v15_check` (embuf/expbuf — *both operands of the compare this
  >   entry names as the whole decision*), `_rsa_recover_em` (nbuf/sbuf/mbuf),
  >   and `bn_mont_modexp_pub`, which took stack locals at 3.12.4. The banked
  >   globals `_rsa_em` / `_rsa_expected` no longer appear in `lib/sigil.cyr`.
  >
  > What is still accurate and still load-bearing: the mechanism, the
  > measurements, the `cbank()` lane analysis, and
  > `tests/server_auth_lane_race.tcyr` as the standing regression guard — it
  > caught every intermediate stage, so it demonstrably detects a relapse.
  >
  > ⚠ Still **open**, and NOT resolved by any of the above: the closing
  > paragraph's wider claim. sigil's PSS lanes were localised in 3.12.6, but a
  > brace-depth scan still finds file-scope banked state elsewhere — `sha256`
  > itself keeps a `cbank()`-banked message schedule (`&W + cbank() * 512`),
  > which is fail-closed for a verify (a corrupted digest mismatches) but is the
  > same structural pattern. Not probed.

  `_agnosai_auth_validate_jwt` now calls `_agnosai_auth_rsa_verify_locked`
  (`src/server/auth.cyr`), which holds a process-global mutex across
  `rsa_pkcs1v15_verify_sha256`. **Do not remove that lock as an optimisation** —
  its header states why, at length, and `tests/server_auth_lane_race.tcyr` fails
  if it goes.

  **The mechanism.** sigil's RSA verify workspace is a set of *file-scope*
  globals indexed by a per-thread lane, and `_rsa_pkcs1v15_check`
  (`lib/sigil.cyr:17887`) ends

  ```
  return ct_eq_bytes(rem, rexp, n_len);
  ```

  where `rem = &_rsa_em + bk*512` and `rexp = &_rsa_expected + bk*512` — **both
  operands of the security decision live in the same shared object**, and the
  comment at `:17780-17789` states the workspace is never wiped per lane.
  `ct_eq_bytes` (`lib/ct.cyr:45-53`) is a plain OR-accumulating compare with no
  structural gate, and `_rsa_recover_em` (`:17847-17872`) performs no PKCS#1
  validation, so that one comparison is the entire decision. A colliding thread
  that leaves a *consistent* valid pair in the lane makes the next thread's
  compare succeed on bytes that were never its own.

  **Lanes collide structurally, and it is not a concurrency threshold.**
  `cbank()` (`lib/sigil.cyr:4403-4418`) assigns
  `(atomic_fetch_add(&_crypto_next_bank, 1) % 63) + 1` — 63 lanes — and **never
  releases one**; there is no decrement anywhere in sigil. The bound is 63
  *lifetime* crypto-touching threads. agnosai has 100 pool workers
  (`server/serve.cyr:117` → `:529` → `lib/sandhi.cyr:14270`/`:14321`), plus a
  detached thread per orchestrator job, plus up to 500 in
  `tools/builtin/load_testing.cyr`.

  **Measured before the fix** — 100 threads × 8000 iterations against a real
  RSA-2048 vector, forged signature = valid signature with one byte flipped so
  the digest, and therefore `rexp`, is identical:

  | | |
  |---|---|
  | forged signatures **accepted** | **888 of 400,000** (~1 in 450) |
  | valid signatures **rejected** | **281,965 of 400,000** (70%) |

  Reproduced independently three times (888 / 1674 / 314) by agents that were
  each trying to *refute* it.

  ⚠ **Pinning every thread to one lane shows zero false accepts and near-total
  false rejects. That is not evidence of fail-closed.** Extreme contention
  corrupts the lane so continuously that no clean snapshot survives to match
  against. The bypass lives in the *realistic* regime — 100 threads over 63
  lanes gives low-multiplicity pair collisions, which is what produces it.
  Anyone re-testing by hammering a single lane will wrongly conclude it is safe.

  **Exposure**: RS256 mode only. `agnosai_auth_config_with_jwt`
  (`auth.cyr:79-85`) sets `ENABLED=1` itself, so **`AGNOSAI_JWT_PUBLIC_KEY`
  alone turns auth on** — `AGNOSAI_AUTH_ENABLED` is not required, which is worth
  knowing because it widens who is affected. Shared-secret mode never reaches
  the RSA path; auth-disabled never reaches it.

  **Capping `AGNOSAI_SERVE_WORKERS` at 63 was considered and rejected**: lanes
  are held for a thread's life so unbounded detached threads wrap anyway, and
  that constant is also sandhi's connection ceiling, so it would surrender 37%
  of concurrent connections for no guarantee.

  Cost of the lock: ~0.85 ms per verify under contention, an
  authenticated-request ceiling around ~1.2k/s on this path only.

  **Owed upstream to sigil**: hold the verify workspace in function-scope
  locals. Verified viable — it needs 1,536 bytes against a **122,880-byte**
  per-fn stack budget (`cyrius/src/frontend/parse_decl.cyr:89`, bisected at
  122,864 B stays / 122,872 B relocates), and sigil's own "function-scope arrays
  are static globals" premise (`:17779`) is **stale**: probed at four recursion
  depths with distinct addresses and 200,000 concurrent passes, zero corruption.

  ⚠ **Wider than JWT, and not yet addressed.** A brace-depth scan finds **62
  file-scope banked globals** in sigil, including the shared bignum engine
  (`_bn_mont_*`, `_bn_exp_*`, `_bn_inv_*`) — so two aliased threads corrupt each
  other even running *different* primitives — and the PSS/ECDSA/Ed25519 lanes
  that `lib/tls_native_hs13.cyr:257-284` routes TLS 1.3 CertificateVerify
  through. agnosai makes outbound HTTPS from `tools/agnos.cyr:244`,
  `tools/remote_registry.cyr:156`, `guarded_fetch.cyr` and up to 500 concurrent
  threads in `load_testing.cyr`, so **TLS peer authentication rides the same
  mechanism**. Structurally identical, not probed.

### Added

- **M12 bite 1 — `llm/inference_queue` is ported, and it was NOT out of scope.**
  The last unported module under `src/`. **18 mutation probes, 18 kills**;
  `tests/llm_inference_queue.tcyr` is **69 assertions**.

  ⚠ **`src/llm/mod.cyr` said this file "defers with that feature", and that was
  wrong twice over.** Wrong on the standing rule — the port is all of
  `rust-old/`, and a cargo feature gate is not a scope boundary — and already
  contradicted by its own sibling: `llm/router` ports its two
  `#[cfg(feature = "hwaccel")]` functions and says so in its header. `[deps.majra]`
  is declared and vendored either way. The note is corrected in place.

  Three forced substitutions, all from the absence of an async runtime, all in
  the module header:

  - **`enqueue` is synchronous**, so `pending()` is 1 the instant it returns.
    The oracle wraps `queue.enqueue(item)` in a `tokio::spawn`, which is why its
    own test sleeps 10 ms before asserting `pending() == 1`. The port's
    guarantee is strictly stronger, so the suite asserts it with **no sleep** —
    that assertion is what pins the substitution.
  - **`spawn_worker` returns a stop handle, not a `JoinHandle`.** `abort()` has
    no equivalent for a detached OS thread. The handle also publishes an
    `EXITED` flag on the way out, because `_stopped` only reads back what
    `_stop` wrote — a worker that ignored the flag entirely passed until the
    thread's own acknowledgement was observable.
  - **The reply is a `chan_new(1)`**, a oneshot that does not consume itself.

  ⚠ **`enqueue` holds a mutex the oracle does not need**, and it is not
  belt-and-braces. majra's `queue_item_new` allocates with `fl_alloc`, which is
  **not thread-safe** — `lib/freelist.cyr` pops the global `_fl_heads` lists
  with plain loads and stores, where `alloc` has a process-wide CAS spinlock
  (`lib/alloc.cyr:28`). The item is built by the *caller*, so `cpq_enqueue`'s
  own mutex does not cover it. That is majra 2.6.1's `relay_receive` bug one
  layer up. The lock fixes agnosai's self-race, not the process-wide hazard.

  ⚠ **`agnosai_map_priority` is a five-arm ladder and must not become
  arithmetic.** The two enums are exact inverses today — agnosai's Background is
  0, majra's is 4 — so `4 - p` passes every assertion and breaks silently the
  first time either side inserts a level. The suite asserts the inversion
  numerically as well as through the mapping, and a `4 - p` mutant is killed by
  the out-of-range case. The ladder is also the bounds check: majra's
  `pq_enqueue` clamps `>= NUM_PRIORITIES` but **not** negatives, so a -1 would
  reach `load64(pq + pri * 8)` and read before the queue.

  ⚠ **The transport is a function pointer, and a survivor is why.** A mutant
  that passed 0 to `settle` instead of the response **survived every
  assertion**, because with no gateway listening only the failure arm is
  reachable and the response is unused there. `agnosai_inference_queue_run_item_with`
  takes the chat call as a `callptr` target, the way `tools/agnos.cyr` does for
  the same reason; nothing is injected in production.

- **M12 bite 2 — `tests/server_routes_sse.tcyr`, a suite that did not exist.**
  All 863 oracle test functions were screened against the Cyrius suites. Two
  flagged, and both were real:

  ⚠ **`src/server/routes/sse.cyr` was the ONE `routes/*` module with no test
  file.** Not a naming artifact: `tests/server_sse.tcyr` covers
  `src/server/sse.cyr`, the event **bus**, and `tests/server_router.tcyr` only
  pins that `GET /api/v1/crews/{id}/stream` reaches `AGN_ROUTE_CREW_STREAM`.
  Nothing asserted anything the handler writes. **31 assertions.**

  The handler streams to a raw fd rather than returning a response object, so it
  is driven against a capture **file** — `sandhi_server_send_chunked_start` and
  `send_chunk` only `write(2)`, so the file holds exactly the bytes a client
  would read. The active-crew case needs the handler on its own thread, and the
  handoff waits for the sender's **receiver count** to reach 2 rather than
  sleeping: a sleep races in the direction that hides the bug, because tearing
  the crew down early sends the handler down the unknown-crew path where every
  assertion still has something to match on the wrong frame.

  Past the oracle: the error frame's wire shape, that `has` is asked **before**
  `subscribe` so probing a nonexistent crew leaves no bus entry behind, and that
  an **uppercase** uuid reaches a crew registered in lowercase instead of
  reporting "crew not found" for a crew that is running.

  ⚠ **`multiple_crews_tracked_independently` had no counterpart either.** Every
  other assertion in `tests/orch_orchestrator.tcyr` runs one crew, so a registry
  with a single slot passed the whole suite. Added; that suite is now **55
  assertions**.

- **M12 bite 3c — the scoring benchmarks, and they confirm an O(n²) sort.**
  All seven `rust-old/benches/scoring.rs` ids are now in `benches/orch.bcyr`.
  The oracle sorts with `sort_by` (pdqsort, O(n log n),
  `rust-old/src/orchestrator/scoring.rs:212`); the port hand-rolls an insertion
  sort (`src/orchestrator/scoring.cyr:294-303`).

  | agents | measured | |
  |---|---|---|
  | 100 | **127.8 µs** | sort ≈ 33% |
  | 300 | **626.0 µs** | predicted 637.8 — **within 2%** |
  | 1000 | **5093.0 µs** | sort ≈ 83% |

  Fitting `a·n + b·n²` to the 100 and 1000 points gives a = 0.863 µs (scoring)
  and b = 0.00421 µs (sort). The 300-agent row is an **independent** point that
  tests that fit rather than being used to build it — two points can be fitted by
  a quadratic but not tested against one. Crossover at **n ≈ 205**.

  ⚠ **A performance note, not a DoS vector — the previous entry said otherwise
  and was wrong.** `AGNOSAI_CREW_MAX_AGENTS` is **100**, enforced at
  `src/server/routes/crews.cyr:353`, so nothing reachable over HTTP passes n=100,
  where the sort is a third of a 128 µs call. The crossover is beyond the cap.
  Only a direct library caller is unbounded. I had repeated the gap analysis's
  "attacker-influenceable agent count" without checking for a cap.

  ⚠ The agents must be **varied**. Identical agents tie on every score, and a
  stable insertion sort walks a tied list in O(n) because the shift loop breaks
  on the first comparison — which is exactly why `rank_agents_16`, 16 identical
  agents, never showed this in two weeks of history.

  **Left open deliberately:** whether to replace the sort. It is a real
  complexity divergence, so CLAUDE.md wants a fix or an ADR — but at the enforced
  cap it costs ~42 µs, and the honest answer may be an ADR recording the bound.

- **M12 bite 3b — the 83 bench gaps are closed.** Six agents, one per
  `benches/*.bcyr`, each compiling and running its own file; every result then
  handed to a separate agent told to assume it was wrong. **All 9 files compile;
  `bench-history.csv` goes 106 → 190 rows.**

  The six adversarial reports are kept verbatim in
  [`docs/development/m12-bench-audit-2026-08-10.md`](docs/development/m12-bench-audit-2026-08-10.md)
  with triage. **Several shipped comments are known-wrong and are listed there
  rather than fixed** — read a file's section before editing its rows.

  Two acted on, and the pair is the point:

  - **FIXED — four fleet placement shapes never reached the sort.**
    `vec_sort_by` (`lib/vec.cyr:340-352`) runs an O(n) already-ordered pre-check
    and returns before `_vec_introsort`. Their comments claimed they guarded the
    score-only-comparator divergence at `src/fleet/placement.cyr:20-23` — with
    which every adjacent compare returns 0, the pre-check still short-circuits,
    and the timing is identical. Structurally blind to the one regression named.

    ⚠ **Reordering the input does not fix it**, which the first attempt did:
    `AGN_PR_INDEX` comes from the scan position (`placement.cyr:283`) and
    survivors are pushed in scan order, so output indices are ascending *by
    construction*. A stride shuffle measured **11.763µs** against **11.722µs** —
    no difference, because there was none to measure. Varying the *scores* over a
    four-tier VRAM sawtooth prices it: **19.53µs against 11.80µs.**

  - **REJECTED — `clock_epoch_secs = 1.318µs` is real.** A verifier called it
    unsourced and implausible, reasoning from `lib/bench.cyr:6`'s documented
    `clock_gettime: ~120ns` and from `uuid_v4_generate` costing 501ns while
    making a real `getrandom`. Measured independently at 2,000,000 iterations:
    **1.315µs**. The implementing agent was right and the verifier — in the
    confident, well-cited style that reads as reliable — was wrong.

    ⚠ **It is `lib/bench.cyr`'s constant that does not hold here**, by 11×,
    because Cyrius issues a raw `syscall(228, …)` rather than taking the vDSO
    path. That constant is what every benchmark in the ecosystem implicitly
    subtracts. The figure is now a measured row (`clock_epoch_secs_baseline`) so
    the arithmetic is falsifiable from the CSV rather than asserted in prose.

  Six of fleet's 19 gaps — the `scoring.rs` set — shipped nowhere; closed
  2026-08-11, see the next entry.

- **The remaining bench gap is mapped, by a seven-way parallel analysis with an
  adversarial audit on each.** Every oracle criterion id classified against the
  tree: **83 real gaps, 10 already covered**, targeting `core`, `orch`, `tools`,
  `server`, and two new files (`fleet.bcyr`, `definitions.bcyr`).

  ⚠ **The audits corrected the analyses in every single group**, and that is the
  part worth keeping: a fabricated "~4,500 string comparisons" figure, a wrong
  allocator model behind three iteration counts (`vec_new_a` preallocates 16
  slots — there is no 1→2→4→8 growth), a `sakshi` log-level trap that does not
  exist (`SK_INFO=3 < SK_DEBUG=4`, so the guard already returns), and the compile
  blocker above. **The raw analyses are not safe to follow verbatim.**

  ⚠ **One claim from the audits did NOT survive my own check, and the roadmap
  records the correction rather than the claim.** An agent reported
  `agnosai_scheduler_load_dag` sorting all DAG keys as a *parity divergence*
  needing a fix or an ADR. Reading `agnosai_scheduler_kahn_sort` shows it sorts
  the zero-in-degree seed itself and sorts each successor list inside its loop —
  both faithful to the oracle — so the caller's key order affects nothing and the
  output is identical either way. It is **redundant work, not a divergence**:
  a per-load insertion sort over every key whose result the next sort discards,
  ~n²/4 comparisons on a 500-task DAG against the oracle's ~0. Left unfixed
  deliberately until the scheduler benchmarks exist, so the deletion can be shown
  rather than argued.

- **M12 bite 3 — `benches/llm.bcyr`, and `benches/` had nothing for `src/llm/`
  at all.** Six `.bcyr` files and not one touched the LLM group, so the routing
  matrix every crew task crosses had no number. All three of
  `rust-old/benches/llm_router.rs`'s criterion groups are reproduced, plus the
  inference queue.

  **The benchmark immediately found a defect in a dependency.** majra's
  `pq_dequeue` pops with `vec_remove(tier, 0)`, which shifts the whole tail — so
  one pop is O(n) in the tier depth and a full drain is **O(n²)**. Measured, mean
  cost of one pop while draining a tier of that depth:

  | tier depth | per pop | ratio |
  |---|---|---|
  | 2,000 | 2.00 µs | — |
  | 4,000 | 4.02 µs | **2.01×** |
  | 8,000 | 7.92 µs | **1.97×** |
  | 16,000 | 15.56 µs | **1.96×** |

  Doubling the depth doubles the per-pop cost, which rules out cache effects. At
  200,000 queued the mean pop was **198.7 µs** and the drain took ~40 s of
  memmove. This is the design case rather than a pathological one — the queue
  exists so background work is *allowed* to accumulate behind interactive work.
  `benches/llm.bcyr` therefore measures the drain at **two** depths, so the slope
  stays legible in `bench-history.csv` instead of averaging into one
  uninterpretable figure. Filed upstream as
  `majra/docs/development/issues/2026-08-10-pq-dequeue-is-linear-so-draining-a-queue-is-quadratic.md`.

- **⚠ THREE OF THE SIX BENCHMARK FILES DID NOT COMPILE, and `cyrius bench` had
  been reporting `5 passed, 3 failed` unnoticed.** `benches/server.bcyr`,
  `benches/orch.bcyr` and `benches/tools.bcyr` all had include lists that had
  gone stale against `src/`, so every benchmark in them — **50 of the 79 in the
  tree**, including the whole 35-shape orchestration set — had stopped running.

  Cyrius resolution is single-pass, callees before callers, so these were hard
  undefined-symbol errors rather than warnings:

  - `src/telemetry/mod.cyr` missing from all three. `llm/hoosh`,
    `orchestrator/crew_runner` and `tools/native` record GenAI spans
    ([ADR 017](docs/adr/017-genai-span-call-sites.md)), which landed after these files were
    last touched.
  - `src/strcase.cyr` missing from `server.bcyr` and `orch.bcyr` —
    `server/prompt_guard.cyr:93` calls `agnosai_str_contains_ci`.
  - the sandbox group and `definitions/loader` missing from `server.bcyr`, since
    `tools/mod` gained a sandbox dependency when `tools/python_tool` landed.
  - `src/server/routes/definitions.cyr` missing from `server.bcyr` —
    `router.cyr` dispatches `AGN_ROUTE_LIST_PRESETS` to
    `agnosai_route_list_presets`, added by M10.

  All three now compile and report. **This is why "never skip benchmarks" is a
  rule about the gate and not only about the numbers**: a benchmark that stops
  compiling reports nothing, and nothing looks exactly like no regression.

  ⚠ **The gate was never silent — it was never invoked.** `cyrius bench` exits
  **1** on a compile error (verified against a deliberately broken `.bcyr`), and
  `scripts/bench-history.sh` runs it under `set -euo pipefail`, so it would have
  aborted rather than recorded partial rows. The last recorded run was
  **2026-08-07**; `src/` moved under `benches/` in the three days after, and
  nothing ran it again. **106 rows recorded on the fixed tree**, against 79
  benchmark shapes — several shapes are parameterised helpers that report more
  than once. The real hole was structural: **`check-clean.sh` did
  not sweep `benches/` and CI never compiled it.** Both fixed —
  `.bcyr` files join the fmt and lint loops, and CI gains a `Benchmarks` step
  that exists for the compile rather than the numbers, since CI timings are too
  noisy to gate on.

  Found by an **adversarial audit** of a benchmark-gap analysis, which flagged it
  as a blocker the analysis itself had not mentioned.

- **M12 bite 7 — the user-facing docs described a binary that no longer exists.**
  `README.md`, `docs/guides/api-reference.md` and `docs/architecture/overview.md`
  all told a reader to run **`agnosai-server`**, the Rust tree's
  `[[bin]]`. The Cyrius build produces one binary, **`agnosai`**
  (`cyrius.cyml` `[build].output` / `[release].bins`). Every instruction in the
  README's Quick Start was a `cargo` command against a tree with no root
  `Cargo.toml`, including a `make check` with no Makefile.

  Corrected across all three, plus: the "Usage as a Library" section now shows a
  `[deps.agnosai]` stanza and Cyrius code (the Rust version is kept in a
  `<details>` for comparison, since it is still the parity oracle); the test-suite
  section shows `cyrius tests tests` and the real numbers; and the project-structure
  block notes that its `[feature: …]` tags are the Rust tree's cargo features and
  **are not scope boundaries here** — every module so tagged is built
  unconditionally.

  ⚠ **`docs/guides/adding-wasm-tools.md`'s `cargo build --target wasm32-wasip1`
  is CORRECT and was left alone.** That instruction is for a third-party tool
  author building against the Rust SDK, which is exactly the surface that stays
  Rust. Sweeping every `cargo` out of the docs would have broken it.

- **M12 bite 6 — the four `cargo-fuzz` targets, as real `fuzz/*.fcyr` harnesses.**
  **3,414 malformed inputs across four harnesses, zero faults.**

  ⚠ **`cyrius fuzz` EXISTS, and a first draft of this work was written around a
  claim that it did not.** So does `cyrius doctest`. Both were asserted absent
  without running bare `cyrius`, which lists them under *Quality*. The standing
  rule — verify a claimed toolchain gap before writing around it — was already in
  memory and was not followed. The draft `tests/fuzz_parsers.tcyr` is deleted in
  favour of the idiomatic form.

  ⚠ **And underneath that, a worse finding: `tests/agnosai.fcyr` — the harness
  `cyrius port` scaffolded on 2026-07-28 — had a `fuzz_main` that returned 0
  without reading its input.** `cyrius fuzz` had been reporting `1 passed` for it
  ever since, for a harness that tested nothing. Nothing in CI ran it either. The
  same was true of `tests/agnosai.bcyr`, a scaffolded `noop` benchmark sitting in
  `tests/` where an audit of `benches/*.bcyr` could not see it — now
  `benches/harness.bcyr`, kept because a measurement floor is genuinely useful,
  with its bench name unchanged so its history stays continuous.

  `fuzz_main(data, len)` mirrors libfuzzer's `fuzz_target!(|data: &[u8]|)`. All
  four oracle targets are the same four lines — arbitrary UTF-8 into a parser,
  result discarded — so the property is *no panic* and the value is entirely in
  the input distribution. `cyrius fuzz` does not generate input, and a random
  generator would be **worse**: a fuzz failure CI cannot reproduce is a failure
  nobody fixes.

  So the corpus is deterministic, built from the two generators that actually
  find parser bugs: a **truncation sweep** (every prefix of a valid document,
  which walks every length check and look-ahead straight off its boundary) and a
  **byte-substitution sweep** (each offset × NUL, `"`, `\`, `{`, `]`, 0xFF),
  plus 25 hand-written JSON-killers — lone surrogates, `1e999999`,
  `9223372036854775808`, an embedded NUL, 64-deep nesting both closed and left
  open.

  ⚠ **It reaches past the oracle.** libfuzzer guards with
  `if let Ok(s) = std::str::from_utf8(data)`, so the Rust targets only ever see
  valid UTF-8. A Cyrius `Str` is bytes and `bayan_json_v_parse_buf` takes
  (ptr, len), so **0xFF reaches the parser here and cannot upstream**.

  ⚠ The suite re-parses the pristine seed after every sweep. A parser can survive
  malformed input by leaving a scratch buffer or a global error slot in a state
  that breaks the *next* caller, and that failure never appears as a crash during
  the sweep. The corpus size is asserted **exactly** rather than as a threshold,
  because a threshold passes while half the corpus quietly stops being generated.

  Found in passing: **`agnosai_crew_from_value` requires `id`**, and it is the
  only required field — `name`, `agents` and `tasks` all default — so a seed
  without one makes the whole sweep re-measure the same early return. The first
  draft of the corpus had exactly that bug.

  Both `cyrius fuzz` and `cyrius doctest` are now gated: `fuzz` as its own CI
  step, `doctest` in `check-clean.sh` over every file carrying a `# >>>` block.

- **M12's implementation work is COMPLETE — nine bites, 2026-08-10/11.** Nothing
  is left under `src/`, `tests/`, `benches/`, `fuzz/` or `examples/`.

  | | |
  |---|---|
  | `src/` modules | **all ported** — `llm/inference_queue` was the last |
  | test suites | **97**, 0 failures |
  | reference coverage | **1561/1561 (100%)** |
  | benchmarks | **10 `.bcyr`**, ~200 rows, all compiling |
  | fuzz harnesses | **4**, 3,414 malformed inputs, 0 faults |
  | doctests | **1** — the only real one under `rust-old/src/` |

  What remains is the WASM **execute** path (needs `wasmtime` installed; the
  manifest half is tested) and five decisions that are the maintainer's, not owed
  work: **D1** mount `rate_limit`, **D2** `"personality": null`, and the three
  opened by this work — **D3** memoize `builtin_presets()`, **D4** `rounds x batch`
  for the bench sweep, **D5** dispatch rows including body serialization.

- **⚠ BENCH DISCONTINUITY 2026-08-11 — nine committed rows changed what they
  measure. Do not compare them across this date.**

  Two fixture corrections in bite 9 were right on the merits and both re-baseline
  existing ids. Recording it because `bench-history.csv` is the proof, and a
  silent re-baseline turns that proof into noise:

  **`benches/server.bcyr`** — the fixture registered ONE tool where
  `rust-old/benches/server.rs` registers two. Fixing it moves four rows by
  50–108%, because `GET /api/v1/tools` and the MCP `tools/list` envelope both
  serialize every registered tool:

  | row | before | after |
  |---|---|---|
  | `route_tools_global` | 2223 ns | ~4500 ns |
  | `route_tools_arena` | 1370 ns | ~2740 ns |
  | `route_mcp_global` | 3883 ns | ~6100 ns |
  | `route_mcp_arena` | 2339 ns | ~3520 ns |

  **`benches/tools.bcyr`** — registry keys were unpadded 6-byte strings and the
  miss key was `"nonexistent"`, so the 5/50/500 sweep conflated registry size
  with key length and the miss probe took a different hash path from the hit.
  Padding the keys and matching the miss key's length re-baselines
  `tool_registry_get_5`, `_get_50`, `_get_500`, `_has_50_hit` and `_has_50_miss`.

  Every other pre-existing row is within ±3%, so the discontinuity is exactly
  these nine and nothing else. Both changes make the rows measure what their
  names claim; neither is a regression.

- **M12 bite 9 — the bench audit's remaining findings, applied file by file.**
  Four agents, one per `.bcyr`, each told the audit had already been refuted three
  times and to verify every finding against the tree before acting.

  ⚠ **The rejections are the valuable part.** They independently re-derived my
  `clock_epoch_secs` rejection, and caught things I had not: the audit says
  "nine `.bcyr`" where there are ten and uses that count as the reason a dropped
  bench line is easy to miss; its "numbers cohere" arithmetic assumes 15 fields
  per agent where the serializer emits 14; and its citation for
  `AgentDefinition`'s derive is wrong in both span and field inventory.

  Escalations acted on:

  - **`_agnosai_assembler_best` memoizes `match_score` where the oracle recomputes
    it**, and it was undocumented. Verified against
    `rust-old/src/definitions/assembler.rs:34-47`: `Iterator::max_by` calls its
    comparator n−1 times and the comparator scores **both** sides, then `.filter`
    scores the winner again — **39 evaluations per member, 195 per 5-member call**,
    plus a deep `.cloned()` per pick. The port scores once per agent and reuses the
    cached value — **20 per member, 100 per call**, returning borrowed pointers.
    Identical results (`match_score` is pure), so this is a module note in
    `src/definitions/assembler.cyr` rather than an ADR — recorded so nobody reads
    `assemble_team_5m_20a` as the oracle's cost. It is roughly half of it.
  - **`lib/bench.cyr:6`'s `clock_gettime: ~120ns` is wrong by 11x** — filed
    upstream as
    `2026-08-11-lib-bench-documents-clock-gettime-at-120ns-it-is-1300ns.md`, with
    eight runs at 2,000,000 iterations and the reason (`lib/chrono.cyr:75` issues a
    raw `syscall(228)`, not the vDSO path libc takes). The other three constants in
    that block check out, so it is one wrong line rather than a stale block.

    ✅ **RESOLVED in cyrius 6.5.19** — the constant is retired for a measured
    `bench_clock_overhead_ns()`. ⚠ **The reason given above is wrong on this
    host**: the vDSO path costs the same as the raw syscall, because the
    clocksource is **hpet** and the vDSO falls back to the syscall. See the
    6.5.19 entry under **Changed**.
  - **`src/server/prompt_guard.cyr` carried a stale number.** Its header claimed a
    clean 4 KB scan costs **273 µs**; the committed row says **352 µs** — 29%
    drift, unnoticed because a figure frozen in a comment has nothing to
    reconcile it against. It now points at the bench id instead.

  Three new open decisions, all in `roadmap.md`, none taken:

  - **D3** — `GET /api/v1/presets` re-parses all 18 documents per request
    (~744 µs parse, ~145 µs serialize, 1,016,776 bytes) for a compile-time
    constant. **Not fixed**, because `rust-old/src/definitions/loader.rs:122`
    does exactly the same, so memoizing would be the divergence. The leak half is
    already gone now that the arena arm is wired.
  - **D4** — every one of the ~200 rows in `bench-history.csv` prints
    `min == max == avg`, by construction: each benchmark takes exactly one
    sample. `lib/bench.cyr:198-211` documents a `rounds x batch` form that would
    give real dispersion; measured run-to-run spread is ~5%, which is exactly what
    a reader currently cannot see. Changing one file makes it incomparable with
    nine others; changing all ten re-baselines the CSV. Raised independently by
    two agents auditing different files.
  - **D5** — the dispatch rows stop at the response object where the oracle's
    `tower` service serializes to bytes.

- **M12 bite 8 — the oracle's shipped WASM fixture is now a conformance test.**
  `tests/tools_wasm.tcyr` loads
  `rust-old/examples/wasm-tools/hello-tool/manifest.json`, transcribed byte for
  byte, through `agnosai_wasm_load_tool_package` — 84 assertions.

  Everything else in that suite parses manifests written for the test. This one
  is what `sdk/agnosai-tool-sdk` actually publishes to tool authors, so it is
  where drift between the SDK's format and this loader would surface — and it is
  the only place `"required": false` arrives from a real published manifest
  rather than from a case constructed to exercise it. Since the SDK stays Rust by
  design, conformance is the only thing agnosai can owe it.

  ⚠ **Only the EXECUTE half stays blocked.** `wasmtime` is not installed here, so
  the `.wasm` beside the manifest is the 8-byte header stub. The manifest parse,
  the `<name>.wasm` resolution, both error policies and the whole result ladder
  need no runtime.

  ⚠ Found while adding it: `_T_ROOT` in that suite is created but never cleaned,
  so its `load_all_tool_packages` count assertions break on anything left behind
  — and the failure ("got 3, expected 2") accuses the counting logic rather than
  the leftover. Cost me a wrong diagnosis before I spotted it; recorded in the
  file so the next reader does not repeat it.

- **M12 bite 5 — the doctest and `examples/`.**

  **`src/core/mod.cyr` now carries a real `# >>>` doctest that `cyrius doctest`
  runs**, plus `tests/core_mod_doctest.tcyr` (13 assertions) at the assertion
  level, since a doctest can only check an exit code. Only **one** of the four
  fenced blocks under `rust-old/src/` is a real doctest, `core/mod.rs`'s — the
  others are ` ```text `, ` ```ignore ` and ` ```yaml `, and `cargo test`
  compiles none of them.

  ⚠ **`cyrius doctest` auto-prepends NOTHING** — not `[deps].stdlib`, not the
  `[deps.NAME]` bundles, not even `lib/str.cyr`, where `cyrius tests` and
  `cyrius bench` prepend all of it. Measured: a doctest calling `str_from` in
  this project reports `undefined function 'str_from'`. That is why **24
  `include` lines** precede two lines of example. cyrius's own `lib/hashmap.cyr`
  doctest is broken by exactly this — it includes `lib/string.cyr` where it needs
  `lib/str.cyr` — and both are filed upstream as
  `2026-08-10-doctest-does-not-auto-prepend-deps-like-tests-and-bench-do.md`.

  ⚠ Every symbol it touches is already covered by `core_agent`/`core_task`/
  `core_crew`, and that is not the point. A doctest asserts that **the example a
  reader is shown still works** — the one test that fails when an API change
  silently invalidates the prose. `src/core/mod.cyr` now carries the block it
  mirrors, with a note to move both together.

  **`examples/simple_crew.cyr`** is the first file in a directory `README.md:35`
  had been documenting for months. It builds and runs against nothing:

  ```
  Crew completed with status: completed
    task output: Analyze the project structure
  ```

  ⚠ **`sdk/agnosai-tool-sdk/` and `rust-old/examples/wasm-tools/` stay Rust, and
  that is a decision, not a deferral.** The SDK is published to *third-party tool
  authors* who compile it to `wasm32-wasip1`; the sandbox executes the result.
  Neither is linked into `build/agnosai`. Rewriting them in Cyrius would break
  every tool author on the published protocol. What agnosai owes is
  **conformance** — that `tools/wasm_tool.cyr` implements the SDK's
  `{"parameters": …}` / `{"result", "success", "error"}` contract, which it does
  and `tests/tools_wasm.tcyr` pins.

  Also corrected: `src/core/mod.cyr` still listed the `#[cfg(feature = "hwaccel")]`
  half of `resource.rs` as "not ported YET". **It shipped with M11** —
  `agnosai_hw_inventory_from_hwaccel` and the whole `TrainingMemoryEstimate` set
  are at `core/resource.cyr:773-910`. `AgentDefinition::personality` is now the
  only carve-out, and it is the one the user actually set.

- **M12 bite 4 — `tests/integration_crew_with_tools.tcyr`.**
  `rust-old/tests/crew_with_tools.rs` sits under `rust-old/tests/` rather than in
  a module's `#[cfg(test)]` block, so **the per-module screen could not see it**
  and it had no Cyrius counterpart. Its two `#[tokio::test]`s are the only
  end-to-end registry → crew → runner exercises in the oracle. **29 assertions.**

  ⚠ The file's name oversells what it couples, in the oracle and therefore here:
  it builds a `ToolRegistry`, registers `EchoTool`, asserts on it — and then calls
  `CrewRunner::new(spec)` **without it**. The crew runs the placeholder path
  throughout, so the tool assertions are standalone and the pipeline assertions
  are about DAG ordering. Reproduced exactly rather than "improved": wiring the
  registry in would test something the oracle does not.

  Past the oracle, the DAG positions are asserted to sum to 3 — a permutation of
  0,1,2. The oracle checks `len == 3` plus two `<` compares, which a runner that
  emitted the same result three times would satisfy.

- **M11 is COMPLETE — all six bites.** Both `hwaccel` halves,
  `tools/python_tool`, `sandbox/wasm`, `tools/wasm_tool`, `tools/wasm_loader`:
  **48 mutation probes, 48 kills**.

  ⚠ **`sandbox/wasm.rs` was not in M11's scope until this work.**
  `src/sandbox/mod.cyr` called it "excluded rather than postponed … WASM as a
  format is an explicit cyrius non-goal", which ADR-006's own 2026-08-07
  correction had already overturned. 521 lines and 11 oracle tests.

  **The transport took three upstream round trips to settle**, all of them
  consumed rather than merely filed:

  - **kavach 3.11.8** — [ADR 006](docs/adr/006-cx-tool-sandbox.md)'s correction
    named kavach's wasmtime backend, and against 3.11.7 that backend was
    hardcoded unavailable, had **no stdin channel**, and reported every guest
    failure as a **success with empty stderr**. The oracle's whole contract is
    JSON on stdin and a meaningful exit code, so a WASM tool that cannot be
    given parameters is not a port of that file. Fixed upstream; agnosai pins
    3.11.8 as a **floor**.
  - **sankoch 2.7.7** — `zip_bound` replaced the copy of sankoch's record layout
    that `definitions/packaging` had been carrying, and `zip_last_error()` makes
    an encrypted archive say so instead of being indistinguishable from
    corruption ([ADR 018](docs/adr/018-sankoch-path-check-on-import.md) point 6).
  - **cyrius 6.5.17** — `distlib`'s self-check compiled a bundle without the
    stdlib leaves it had just written to the sidecar, which made kavach's
    release CI red for a reason that was not in kavach.

  [ADR 019](docs/adr/019-wasm-tools-spawn-wasmtime-directly.md) records the
  transport, including the direct-`wasmtime` decision it superseded within a day
  once 3.11.8 landed.

  Behaviours pinned that no oracle test reaches:

  - **The accelerator mapping is not a pass-through**, though it reads like one:
    `ACCEL_TPU = 8` against `AGNOSAI_ACCEL_TPU = 5`, so a bare cast is right five
    times of six and silently turns every TPU into an Intel NPU.
  - **`suggest_quantization`'s argument order is flipped** against the oracle;
    Cyrius types neither parameter, so passing them straight through compiles
    and reads a parameter count as a registry struct. That mutant segfaults.
  - **`fits_in` sums device VRAM only**, so 512 GB of RAM with no GPU fits
    nothing — and ai-hwaccel answers training memory in **x1000 fixed point**,
    so storing it raw makes a 14 GB model read 14000 while every ordering
    assertion still passes.
  - **A timeout that reports exit 0 is still a timeout.** The only input where
    the classifier's first two checks are distinguishable, and the one that
    matters: reversed, a run killed at the deadline reads as a clean success.
  - **A guest's own `proc_exit(n)` survives** — mapping every non-zero code onto
    the trap sentinel would erase it.
  - **`load_module` validates magic AND version**, because a CLI cannot
    pre-compile: a magic-only check accepts a version-2 module and defers the
    failure to an exec-time trap.
  - **The `.wasm` filename comes from the manifest's name, not the
    directory's** — every oracle fixture has the two agree.
  - **`load_tool_package` propagates and `load_all_tool_packages` swallows**, so
    one broken package in a directory is a count that is quietly one lower.
  - **`version` defaults to `"0.0.0"`, `parameters` to empty, `name` and
    `description` are required** — four fields, three policies.
  - **Non-JSON stdout is untrimmed here and trimmed in `python_tool`.** The two
    result ladders are otherwise the same decoder; that is the second instance,
    and CLAUDE.md says extract at the third.

  ⚠ **`wasmtime` is a host requirement and is not installed on this box**, so
  the end-to-end arm is guarded. Everything else is reachable without it,
  because each piece was split out of its caller for exactly that reason — and
  a mutant emitting the bare parameter map instead of the SDK's
  `{"parameters": …}` wrapper survived every assertion until the marshalling was
  split out too.

- **M11, three of six bites: the `hwaccel` halves of `llm/router` and
  `core/resource`, and `tools/python_tool`.** 25 mutation probes, 25 kills.

  ⚠ **The accelerator-type mapping is not a pass-through, and it looks like
  one.** ai-hwaccel numbers `ACCEL_TPU = 8` while `AGNOSAI_ACCEL_TPU = 5`;
  CPU/CUDA/ROCm/Metal/Vulkan are 0-4 on both sides, so a bare cast is right five
  times out of six and silently turns every TPU into an Intel NPU. All thirteen
  unmapped ai-hwaccel variants clamp to CPU — deliberately, so an NPU still
  matches CPU work and never matches a GPU requirement it cannot serve.

  ⚠ **`suggest_quantization`'s argument order is FLIPPED against the oracle.**
  Rust takes `(model_params, registry)` and ai-hwaccel's `reg_suggest_quant`
  takes `(registry, model_params)`. Cyrius types neither parameter, so passing
  them straight through compiles and then reads a parameter count as a registry
  struct — the mutant for it segfaults rather than failing an assertion.

  ⚠ **`HardwareInventory::from_hwaccel` does not make the CPU profile a
  device.** It is diverted into `memory_total_mb` + `cpu_cores` and skipped, so
  a CPU-only registry yields zero devices — and `index` is the *registry*
  position, not a running device count, so the first real device is index 1 with
  a gap at 0. `TrainingMemoryEstimate::fits_in` then sums **device VRAM only**:
  a machine with 512 GB of RAM and no GPU fits nothing, which is correct and is
  one field away from being wrong.

  Units are the other trap: ai-hwaccel answers training memory in **x1000
  fixed-point** integers, so storing them raw makes a 14 GB model read 14000 —
  and every ordering assertion still passes, because the scale is uniform.

  `python_tool` is an adapter over `src/sandbox/python.cyr`, which M7 already
  ported whole. ⚠ **Seven of its ten oracle tests never call
  `PythonTool::execute`** — they re-implement the result ladder inline against
  serde_json and assert on that copy, so upstream they would pass against a
  completely broken `execute`, which has no end-to-end test at all. The ladder
  is split into `agnosai_python_tool_output_of` so those assertions run against
  the real code, and an end-to-end test against a live `python3` is added.
  Reproduced rather than fixed: a non-zero exit **shadows** the structured error
  the shipped bridge actually sends, `success` defaults to true even when it is
  not a boolean, an absent `result` means the whole value but a present JSON
  null does not, non-JSON stdout is a **success**, and the sandbox error prefix
  **doubles** (`"python sandbox error: sandbox error: …"`).

  Two stale exclusions retired with this work: `src/tools/mod.cyr`'s "deferred
  with their features … WASM as a format is an explicit cyrius non-goal", and
  the matching claim in `src/core/resource.cyr` and `src/llm/router.cyr`.

- **M10 `definitions` is COMPLETE — all six modules, all 1,460 oracle lines.**
  `versioning`, `assembler`, `k8s_crd`, `loader`, the eighteen built-in presets
  and `packaging`: **560 assertions** across six suites, all 43 of the oracle's
  tests for the group plus a large margin past them.

  Both of the group's long-standing blockers turned out to be already closed.
  YAML came from bayan's parser, and ZIP needed **one line** — `"sankoch"` in
  `cyrius.cyml`'s `[deps].stdlib` — rather than the "~250 line upstream ask to
  sankoch" the roadmap had predicted; `lib/sankoch.cyr` already shipped 26
  `zip_*` fns.

  **`GET /api/v1/presets` answers eighteen presets instead of `[]`**, and the
  handler moved from `server/routes/tools.cyr` to a new
  `server/routes/definitions.cyr`, which is where the oracle keeps it.

  ⚠ **The old `[]` was not a stub, it was a misread.** The oracle's handler is
  cfg-gated on the `definitions` feature and `default = []` leaves it off, so
  the *default cargo build* really does answer `[]` — and its own test asserts
  that. But this port has no cargo features and ships the whole crate, so the
  populated arm is the true one. `tests/server_router.tcyr`'s assertion was
  inverted along with the handler. Reading a feature-gated oracle test as the
  parity target is how a route stays empty for nine milestones.

  Behaviours pinned that no oracle test reaches:

  - **JSON and YAML failures use DIFFERENT error variants.** `load_from_json`
    raises `Serialization`, `load_from_yaml` raises `InvalidDefinition` — the
    oracle's two `map_err` arms — and its tests only check `is_err()`.
    Collapsing them into one would pass everything upstream.
  - **The extension comes from the file NAME, not the path.** `/tmp/run.d/readme`
    has a dot in a *directory* component; a last-dot-in-the-path scan calls its
    extension `d/readme`, skips the file, and looks correct until someone stores
    definitions under a dotted directory. A dotfile (`.json`) has no extension
    at all, and `.hidden.json` does.
  - **A missing directory is an error, not an empty result.** `dir_list` returns
    an empty vec for a directory it cannot open, so without an explicit `is_dir`
    guard `load_all_from_dir("/no-such-dir")` is indistinguishable from loading
    an empty one.
  - **A preset is all-or-nothing.** `PresetSpec` has no `Option` and no
    `#[serde(default)]`, so one malformed agent rejects the whole document —
    the opposite of `agnosai_agent_from_value` (skips a bad `tools` entry) and
    of `k8s_crd` (skips a bad agent). Copying either would silently truncate a
    team.
  - **`.json` only for presets**, where the definition walk also takes `.yaml`
    and `.yml`; and `load_preset_from_file` checks no extension at all, so JSON
    in a `.yaml` file loads through the direct entry and is invisible to the
    directory walk.
  - **The preset ORDER is domain by domain, lean → standard → large** — not the
    alphabetical order a glob-driven generator produces. `builtin_presets()`
    returns a `Vec`, so position is observable, and no oracle assertion looks at
    it. Every index is asserted.

  **`agnosai_str_eq_ci` / `_contains_ci` / `_contains_ci_cstr` moved to a new
  `src/strcase.cyr`** — extracted at the third caller, which is when CLAUDE.md
  says to (`server/prompt_guard`, `fleet/cost_planning`,
  `definitions/assembler`). No oracle counterpart; Rust gets these from
  `eq_ignore_ascii_case`. Nothing there allocates, which is the point — every
  caller is inside a per-candidate loop. 41 assertions.

  **`src/definitions/presets_data.cyr` is generated and committed.** The oracle
  embeds the eighteen documents with `include_str!`; Cyrius `include` is textual
  and takes a path to *source*, so `./scripts/gen-presets.sh` turns
  `src/presets/*.json` into source. `scripts/check-clean.sh` now runs the
  generator's `--check`, so an edited preset with a stale embed fails the
  cleanliness gate instead of shipping.

  ⚠ **`cyrius fmt` reindents inside a multi-line string literal and the spaces
  land in the string.** A trailing backslash continues a literal and keeps the
  newline; fmt then applies the statement's indentation to the continuation
  line. It silently reindented a YAML fixture in
  `tests/definitions_loader.tcyr` — a nesting change — and turned a passing
  suite red on a file nobody had edited except by formatting it. Filed as
  `cyrius/docs/development/issues/2026-08-09-cyrius-fmt-reindents-inside-multi-line-string-literals.md`.
  Two workarounds are in the tree until it is fixed: every long fixture is
  assembled from short literals rather than continued, and the presets generator
  pipes its own output through `cyrius fmt` (breaking lines only at JSON token
  boundaries) so the two `--check` gates stop contradicting each other.

  ⚠ **`builtin_presets`'s `filter_map(.ok())` is reproduced, so a document that
  fails to parse is DROPPED and the array comes back short** — 200 OK, no log.
  That is unobservable against the real embed, where all eighteen parse, and a
  mutation swapping "skip the failure" for "stop at the failure" survived every
  assertion. `_agnosai_builtin_presets_from(count, json_fp)` exists to give the
  test an accessor with one broken document; the seam is there for that reason
  and the header says so.

- **`definitions/packaging` — `.agpkg` ZIP bundles, on sankoch instead of the
  `zip` crate.** 127 assertions, 14 mutation probes, 14 kills. It is the only
  module in the group that swaps a dependency rather than porting logic, and the
  two libraries disagree in seven places — every one of them recorded in
  [ADR 018](docs/adr/018-sankoch-path-check-on-import.md) and every one stricter
  than the oracle:

  ⚠ **The oracle's 1 MiB zip-bomb guard does not actually bound anything.**
  `file.size()` is attacker-written central-directory metadata and the `zip`
  crate never checks it against the decompressed stream — so an archive
  declaring `uncompressed_size = 10` passes the guard and then inflates
  unbounded into RAM, CRC and all, because the attacker wrote the CRC too. The
  CHANGELOG's own earlier claim of "decompression bomb protection, 1 MiB per
  file" was therefore advisory upstream. sankoch bounds the output at `dst_cap`
  and requires the produced length *and* the CRC to match, so the cap is real
  here.

  ⚠ **An `agent_key` with a `..` component now fails export loudly.** The oracle
  interpolates the key raw into `definitions/<key>.json`, writes it happily, and
  then drops it on the way back in via its own `..` filter — a definition that
  vanishes silently between export and import. sankoch's writer refuses the name.

  Reproduced exactly rather than tidied, none of it covered by an oracle test:
  `MAX_ENTRIES` is `>` so exactly 100 entries is accepted, and the count is
  **unique names** (`archive.len()` is an `IndexMap`), so 101 records under 100
  names still imports; a duplicate name resolves to the **last** record, because
  the map replaces in place; a `..` member is a **substring** test and a
  **silent skip**, so `definitions/a..b.json` is dropped and the import still
  succeeds; an undecodable member is skipped, not an error; `by_name` failing is
  `InvalidDefinition` for the manifest and `Other` for a definition; a decode
  failure is `Io`, not either; and `start_file` and `finish` failures carry two
  **different** messages.

  ⚠ **A duplicate `agent_key` fails export**, because zip 8 rejects a duplicate
  member name and sankoch does not — so the port checks. Without it, export
  succeeds where the oracle errors and the archive imports as two definitions.

  **sankoch's writer never allocates and has no sizing API**, so `export`
  computes an upper bound instead of guessing and retrying. That is only sound
  because DEFLATE degrades to STORE rather than growing a member — and the test
  writes every member STORED into exactly the bound, because checking a bound
  against *compressed* output proves nothing: a bound that dropped the
  per-member header overhead entirely still cleared the deflated archive.

- **The GenAI span call sites are wired** — `llm/hoosh` (inference, CLIENT),
  `tools/native`'s vtable dispatch (tool execution, INTERNAL) and
  `orchestrator/crew_runner` (crew run, INTERNAL). **26 assertions** in
  `tests/telemetry_wiring.tcyr`, three mutation-verified.

  ⚠ **A deliberate divergence from `rust-old`, recorded as
  [ADR 017](docs/adr/017-genai-span-call-sites.md)** and flagged before it was
  made. The oracle defines the four GenAI span helpers and **never calls
  them** — zero call sites outside `genai.rs`'s own doc comment, and the only
  `telemetry::` use in the whole oracle is `main.rs`'s `init_tracing`. That is
  coherent for a *library*, where a downstream Rust consumer instruments its own
  calls; it is not coherent for this port, which is a binary. If `src/` does not
  create spans, nothing ever will: the exporter runs against a permanently empty
  ring, `genai`'s fifteen attributes describe spans nobody constructs, and
  `OTEL_EXPORTER_OTLP_ENDPOINT` becomes a setting that only changes the stderr
  format — which looks like it works.

  The exporter is reached through a **process global** set by `init_tracing`,
  mirroring `tracing::subscriber::set_global_default`; threading a handle from
  `main` down into `hoosh.cyr` would be larger and less faithful. Shutdown
  clears it, so a span created during shutdown cannot enqueue into a stopped
  exporter.

  ⚠ **The first draft of the tool site turned a tool's `0` output into a
  crash.** `agnosai_tool_output_success(0)` dereferences null, and the old code
  passed 0 through harmlessly. Caught by an end-to-end run, not by review; the
  null check now precedes the status read, and a 0 output records as an ERROR
  span. Instrumenting a chokepoint must not change what the chokepoint does.

  Also pinned: a *failing* tool records ERROR, not OK (a backend's error rate is
  what an operator alerts on); a crew span closes on **every** exit including
  the DAG-cycle error return, the same discipline the `crews_active` gauge
  follows; a CANCELLED crew folds to ERROR rather than OK, because OTLP has two
  states where the oracle's `CrewStatus` has three; and with telemetry off —
  the common deployment — every call site still runs and is harmless.

  `tests/telemetry_wiring.tcyr` is the only suite that proves any of this:
  `telemetry_otlp` drives the ring directly and `telemetry_mod` drives the
  formatter directly, and both stay green with every call site deleted.

- **`telemetry/otlp`** — OTLP/HTTP+JSON span export: encoder, ring and
  background exporter. **115 assertions**, twelve mutation-verified.
  **No oracle module**: `rust-old` gets OTLP from
  `hoosh::telemetry::init_otel`, an OpenTelemetry SDK behind a Rust crate, and
  there is no Cyrius OTel SDK. The shape follows hoosh's own `src/lib/otlp.cyr`
  — **the one place ADR 003's remote-HTTP seam does not apply**, because export
  is in-process and there is nothing to call hoosh *for*. Copying its encoder is
  not linking hoosh.

  Encoder, ring and exporter. The encoder is split from everything else so the
  wire format is testable byte-for-byte without a collector — which matters
  because **a collector accepts or silently drops a batch, with no local symptom
  either way**. So the byte layout is the parity surface, asserted literally
  rather than by shape.

  The two rules of the protobuf JSON mapping that bite, both silent failures:

  - **64-bit integers are STRINGS** — `"intValue":"42"`, and every
    `*TimeUnixNano`. IEEE-754 doubles cannot hold the int64 range, so a bare
    number is valid JSON and invalid OTLP. `kind` and `status.code` are
    genuinely bare numbers (enums, not int64), and that distinction is pinned
    too.
  - **Ids are fixed-width lowercase hex** — 32 for a trace id, 16 for a span
    id, zero-padded. The collector parses by length.

  ⚠ **hoosh's `_otlp_path` hardcodes offset 7** to skip `http://`. On an
  `https://` URL that lands on the second `/` of `//` and answers `/` — a POST
  to the collector root, which 404s. hoosh never hits it because that path only
  ever sees `http://`; this port takes the offset from the scheme, and the
  mutation reproducing hoosh's shape fails two assertions.

  **The ring keeps two arenas.** Every span fragment is a fresh allocation and
  the default allocator never frees — ten spans a second is ~14 MB an hour that
  never comes back, which hoosh accepts and this port's own "thread the `_a`
  variants" principle does not. Fragments live in an arena written only under
  the ring lock and reset by `drain`; the batch document lives in an arena
  private to the draining thread and reset at the top of each drain, which makes
  the lifetime rule one sentence — *a document is valid until the next drain*,
  exactly the POST window.

  ⚠ **Overflow is counted.** hoosh's ring overwrites the oldest span and counts
  nothing, so loss is invisible. This one keeps the same drop-oldest policy (a
  full ring means the exporter is behind, and the newest spans are the ones
  being looked at) but reports the count, per the port's no-silent-truncation
  rule.

  **Identity is the caller's, never sakshi's** — the decision this bite owed.
  sakshi's trace id is a process global, so under `run_pooled` two concurrent
  requests share one and the traces interleave into nonsense. Rather than paper
  over it, the export path never reads sakshi's trace context: `enqueue` takes
  an explicit context and correlation is the caller's job. Reentrant by
  construction, no upstream change, honest about where the responsibility sits.
  A real OTel library would supply the thread-local; see the roadmap note.

  ⚠ **The exporter's sleep is sliced, and that was a measured bug, not a
  precaution.** One `sleep_ms(interval)` means `stop` goes unnoticed for up to a
  full second — and because `syscall(60, ..)` exits one *thread*, a detached
  exporter still sleeping holds the whole process open. `tests/telemetry_mod.tcyr`
  took **1004 ms**; sliced at 25 ms it takes **26 ms**, and a SIGTERM shutdown
  loses its one-second tail. Found because a mutation of `shutdown` hung instead
  of failing.

  Two smaller deliberate calls: the POST goes through `sandhi_http_post` for both
  schemes rather than hoosh's raw socket for `http://` (hoosh needs that because
  its exporter thread owns no sigil crypto bank; agnosai does not), and the SSRF
  guard is **deliberately not applied** — the endpoint is operator configuration
  and the standard deployment is a `localhost` sidecar, which the guard blocks by
  design.

  Also pinned: absent attributes are **omitted, not null** (OTLP has no null
  attribute value and a collector reading one drops the span), including the
  comma placement that omission has to get right when the *first* possible
  attribute is the absent one; `SpanKind` CLIENT for inference and INTERNAL for
  local work, because a backend's service-dependency graph is built from
  CLIENT/SERVER pairs; and the all-zero trace id, which is what
  `sakshi_trace_id_hi`/`_lo` answer before anything calls `sakshi_trace_set_128`
  and which W3C declares invalid.

- **`telemetry/genai`** — OpenTelemetry GenAI semantic-convention span helpers
  (v1.37+): the 15 attribute constants, `inference_span`, `tool_span`,
  `crew_span` and `record_usage`. **64 assertions**, all 7 oracle tests plus 57
  past them, five mutation-verified. `telemetry/` is now **complete at source
  level** — both oracle files ported.

  **`tracing::Span` becomes a held attribute record.** The oracle returns a
  handle whose attributes a subscriber reads at close and which `record_usage`
  writes into later; sakshi's spans are a fixed-depth stack of name + timing
  with no attribute channel. So the two halves are split — attributes live in a
  record this module owns (what the OTLP exporter will encode), scope is
  sakshi's stack, entered explicitly.

  ⚠ **Creating a span does not enter it.** `tracing::info_span!` builds; entering
  is a separate `.entered()`. The oracle's doc says "the span will be active
  until dropped" but its own example never enters, so the port follows the code.
  Mutation-verified against a constructor that pushes.

  Four of the oracle's seven tests are **vacuous** —
  `assert!(span.is_disabled() || !span.is_disabled())` is a tautology, and three
  more are `let _ = span; // Verify creation doesn't panic`. The remaining three
  check only the *shape* of the constants. So the constants are exposed as the
  key for real accessors (`span_attr_str` / `span_attr_int`) rather than left
  decorative, and what is pinned past the oracle is:

  - **`crew_span` sets no `gen_ai.operation.name`** while the other two do, so a
    backend grouping by operation sees crew runs as unclassified. The oracle
    omits it; filling it in would invent a wire value.
  - **Unset is distinguishable from zero.** The three `tracing::field::Empty`
    fields read back unset, not 0 — otherwise "no usage recorded" and "recorded
    0 tokens" would be the same span.
  - A **declared-but-Empty** field and an **undeclared** one both read absent,
    because the wire carries neither.
  - The two prefix families **partition** the set — the oracle checks each
    separately, so an attribute in both lists passes both its tests.
  - Every key is asserted by **exact spelling**; `starts_with("gen_ai.")` passes
    for a typo'd suffix, and these are names an OTLP backend matches literally.
  - `agnosai.crew.task_count` is spelled **inline** in the oracle's `crew_span!`
    rather than in `attrs`, so no oracle test covers it at all.

- **`telemetry/mod`** — the OTLP configuration surface and, for the first time,
  **the oracle's JSON log format**. `DEFAULT_SERVICE_NAME`, `service_name`,
  `otlp_endpoint_from_env`, `TracingGuard`, `init_tracing`, plus the formatter
  behind it. **84 assertions**, all 4 oracle tests plus 80 past them, seven
  mutation-verified. `./build/agnosai` now emits:

  ```
  {"timestamp":"2026-08-09T06:55:17.625833Z","level":"INFO","fields":{"message":"AgnosAI server starting"},"target":"agnosai"}
  ```

  ⚠ **The "sakshi has no JSON output mode" divergence was false and is
  retired.** `src/main.cyr` carried it as stated divergence #3 and `roadmap.md`
  M9 planned an upstream filing for it. Neither survived reading the library:
  **`sakshi_set_emit_hook` (`lib/sakshi.cyr:1038`) routes every event through a
  caller-supplied formatter**, and its own doc comment names this exact use.
  The formatter belonged here all along; nothing was filed for it.

  All four oracle tests are **vacuous** — one asserts a constant against its own
  literal, the other three call something and discard the result. So the parity
  surface tested here is the **byte layout of the line**, because a formatter
  emitting valid-but-different JSON would pass every oracle test and break every
  log shipper. Pinned: `tracing_subscriber`'s exact key order; RFC 3339 with
  zero-padded microseconds across leap years, century leap years and second
  rollover; escaping of quote, backslash, newline, tab and control bytes as
  lowercase `\u00xx`; raw UTF-8 passed through; embedded NULs escaped rather
  than terminating the message; and truncation that cannot lose the JSON tail.

  Three things the hook deliberately does **not** do, each stated at its site:
  it ignores the hook's `ts` (monotonic) and reads `clock_epoch_ns()` instead,
  because the oracle's timestamp is wall clock; it **drops span enter/exit
  events**, because `fmt::layer().json()` emits none without `with_span_events`;
  and it allocates nothing — `iso8601()` and `epoch_to_date()` both allocate,
  and this runs once per log line on a no-free bump allocator.

  ⚠ **`RUST_LOG` means different things in the oracle's two init paths**, and
  both are reproduced. `main.rs`'s non-otel branch is
  `from_default_env().add_directive("agnosai=info")` — the added directive wins,
  so the level is **pinned at INFO** and `RUST_LOG` cannot change it.
  `init_tracing`'s is `try_from_default_env().unwrap_or_else(…)` — a set
  `RUST_LOG` **replaces** the default. `main` runs the otel-enabled path because
  it is the superset; that choice, and its one visible consequence, is stated in
  `src/main.cyr`'s header.

  ⚠ **stderr is JSON only when OTLP is OFF.** The oracle's OTLP branch builds
  `fmt::layer().with_writer(stderr)` — the *text* formatter — and only the
  non-OTLP branch calls `.json()`. Reproduced rather than smoothed over: an
  operator whose shipper parses JSON needs to know that enabling the collector
  breaks it.

  ⚠ `EnvFilter` reduces to sakshi's single process-wide level. A bare level and
  `agnosai=<level>` are honoured; **a multi-target `RUST_LOG` is refused
  outright, not half-applied** — applying one directive globally would silently
  raise verbosity for a target the operator had turned down.

- **`fleet/` is COMPLETE** — `topology` was the last of the oracle's twelve
  modules. All 4,443 oracle lines are ported, with **721 assertions** across
  eleven suites covering all **140** oracle test fns (137 `#[test]` plus
  `discovery`'s 3 `#[tokio::test]`).

- **`fleet/topology`** — NVLink/XGMI-aware placement scoring. `InterconnectType`
  / `NodeTopology` / `DeviceLink`, `topology_score` and
  `supports_tensor_parallel`. **39 assertions**, all 6 oracle tests plus 33 past
  them, six mutation-verified.

  Two of the oracle's own defects are reproduced rather than quietly fixed:

  - **`topology_score` is not bounded by 1.0**, whatever its doc comment says.
    Links are never validated against the inventory's device indices and never
    deduplicated, so two GPUs — one pair — with three high-bandwidth links
    score **3.0**. Every oracle test happens to use a consistent link set, so
    none of them can see it. Clamping would be a silent divergence; a caller
    ranking nodes by this number needs to know it can exceed 1.
  - **`supports_tensor_parallel` ignores which devices a link connects.** It
    counts high-bandwidth links against `device_count - 1`, the ring minimum,
    so three links all between devices 0 and 1 "support" a four-way ring.

  Also pinned: the 50 GB/s threshold is inclusive (`>=`), so exactly 50 counts
  and 49.99 does not; `gpu_count <= 1` means a **CPU-only** node scores 1.0
  rather than being penalised, and that same guard is what keeps
  `device_count - 1` from going negative; CPU devices are excluded from the pair
  count, so listing a node's cores as devices must not dilute its score; and
  `interconnect` never decides anything — a 600 GB/s PCIe link counts while a
  1 GB/s NVLink does not, which no oracle test can distinguish because all of
  them use 600 GB/s NVLink.

  ⚠ The `total_pairs == 0` guard is **unreachable** — it is reached only when
  `gpu_count >= 2`, where `n(n-1)/2 >= 1`. Kept for oracle shape and recorded
  in `roadmap.md` section E.

- **`fleet/federation`** — multi-cluster federation: coordinator election,
  cluster discovery and per-cluster health. `FederationRole` / `ClusterStatus` /
  `ClusterInfo` / `ElectionState` / `FederationConfig` / `FederationManager`:
  heartbeat, check_liveness, start_election, declare_coordinator,
  elect_by_lowest_id, coordinator, is_coordinator, role, term, clusters,
  online_clusters, cluster_count, evict_offline. **119 assertions**, all 19
  oracle tests plus 100 past them, nine mutation-verified.

  - **`check_liveness` compares `elapsed > timeout`; `NodeRegistry`'s sweep
    compares `>=`.** The two sweeps look alike and are not, so porting one from
    the other introduces an off-by-one at the boundary. For the same reason the
    comparison runs in **nanoseconds** — truncating elapsed to whole
    milliseconds first would delay every demotion by up to 1 ms and turn a
    strict `>` into an accidental `>=`.
  - **`declare_coordinator` accepts an equal OR future term.** The guard
    refuses only a *stale* one, so a peer declaring at a higher term advances
    this cluster's term with no election running here. Raft-inspired, not Raft:
    no vote, no log, no quorum. The oracle tests stale and current, never
    future.
  - **A heartbeat does not reset an existing cluster's role.** `or_insert_with`
    seeds Follower and the four assignments after it do not touch `role`, so a
    coordinator that beats stays Coordinator — re-seeding would silently demote
    the leader on its next beat.
  - **`elect_by_lowest_id` excludes Suspect and Offline peers and cannot
    fail.** Self is always a candidate, so the oracle's `None` arm is
    unreachable and a lone cluster elects itself while reporting zero known
    clusters. Ids order by **byte** — `"Zeta"` beats `"alpha"`.
  - **`start_election` clears a good coordinator**, abandoning a leader even if
    the election never completes.
  - **`evict_offline` needs BOTH Offline and age**, and measures age from the
    last heartbeat rather than from when the cluster went Offline — so the
    offline timeout is already spent inside the TTL, and flipping an ancient
    Suspect cluster to Offline evicts it on the very next call.

  The oracle sorts the candidate list, dedups it and takes `first()`; the port
  takes the **minimum** directly. Observably identical — `dedup` cannot change a
  first element and the sorted list is discarded — and it allocates nothing on
  the no-free global bump.

  `ClusterInfo`'s fields are all `pub` upstream and the oracle's own tests write
  `status` and `last_heartbeat_instant` to drive liveness without sleeping.
  Both are ported as setters, so every liveness and eviction assertion here is
  **deterministic** rather than sleep-timed — unlike `fleet/registry`, which
  still sleeps.

  ⚠ Three config fields — `endpoint`, `seeds`, `election_timeout` — are stored
  and **never read by any method**, exactly as upstream. The oracle logs
  `seeds.len()` once at startup and its `election_timeout` doc claims a
  randomization that nothing performs. Carried rather than dropped so a caller
  that owns the election timer has one place to configure it.

  ⚠ The strict `>` is **documented, not pinned**: discriminating `>` from `>=`
  needs elapsed to land exactly on the threshold, and the sweep reads its own
  `clock_now_ns()` after a test sets the instant, so the boundary is
  unreachable from outside. Both directions are asserted with a 1 ms margin and
  the gap is recorded in `roadmap.md` section E.

- **`fleet/coordinator`** — the top of the fleet group: crew fan-out, result
  aggregation and failover. `FleetTask` / `FleetTaskStatus` / `FailoverAction`
  and `FleetCoordinator`: fan_out, task, task_count, task_completed,
  task_failed, tasks_for_node, is_complete, completion_pct,
  pending_reassignment, reassign, state_manager. **81 assertions**, all 17
  oracle tests plus 64 past them.

  Four behaviours the oracle's own tests structurally cannot reach:

  - **`max_retries` allows one fewer retry than it reads.** `task_failed`
    increments the counter *before* testing `count < max_retries`, so the
    default 3 gives Retry on failures 1 and 2 and Exhausted on failure 3 —
    **two** retries. The oracle pins Retry at one end and Exhausted at the
    other but never crosses a boundary, so a `<=` implementation passes every
    oracle test. Mutation-verified: `<` → `<=` fails seven named assertions,
    and `with_max_retries(1)` exhausts on the very first failure.
  - **`fan_out` provisions every node for the MAX tasks-per-node**, not its own
    count, because `CrewStateManager::create_run` takes a uniform figure. Two
    tasks on node-a and one on node-b leaves *both* recorded as owing 2. The
    oracle states this in a comment and asserts nothing; mutation-verified
    against a min-per-node implementation.
  - **`reassign` revives a terminal task and keeps its retry count.** It is
    unconditional on status, so a Failed task returns to Assigned — and because
    the counter is not reset, the very next failure exhausts it again.
  - **"Complete" is not "succeeded."** An empty coordinator is complete;
    a run with one Completed and one terminally Failed task is complete with
    `completion_pct` at 1/2. `Reassigned` is deliberately **not** terminal — it
    is a task awaiting another node, so it keeps `is_complete` false.

  An unassigned task is stored `Pending` rather than dropped, which is what
  lets a later `reassign` place it; `tasks_for_node` therefore has to skip the
  0 node pointer, and dropping that guard **segfaults** rather than
  miscounting (mutation-verified).

  `plan_sharding` is `#[cfg(feature = "hwaccel")]` upstream and is **ported
  anyway** — the full-port mandate covers feature-gated code and
  `[deps.ai-hwaccel]` is already vendored. It stays a one-line delegation to
  `reg_plan_sharding`, so all three oracle `hwaccel_tests` port directly on
  ai-hwaccel's own `registry_from_profiles`. Because a one-line delegation can
  only break by dropping or transposing an argument — and the oracle's three
  tests each use a *different* registry, so a hardcoded quant would pass all
  three — one assertion holds the registry and model fixed and varies only the
  quantization.

  ⚠ `_agnosai_fc_evict_completed` reproduces the oracle's shape exactly: it
  checks the cap *before* inserting and removes **all** terminal tasks rather
  than the oldest, so a coordinator holding 10,000 live tasks evicts nothing
  and grows past `MAX_RETAINED_TASKS`. A retention floor, not a ceiling — the
  same shape `CrewStateManager` already carries.

- **`fleet/state`** — distributed crew state with barrier sync and checkpoints
  (backfilled: the module shipped in `15758f0` without a CHANGELOG entry).
  `DistributedCrewState` / `NodeProgress` / `CrewStateManager`: create_run, get,
  report_progress, reach_barrier, force_barrier, remove_node, checkpoint,
  active_runs, overall_progress. **95 assertions**, all 17 oracle tests plus 78
  past them.

  Two Rust enums carry payloads and Cyrius has no sum types, so the encoding is
  where the risk lives:

  - **`CrewPhase` is a tag plus a payload `Str`**, and equality must compare
    both — `WaitingBarrier("a")` is not `WaitingBarrier("b")`, and `remove_node`
    depends on exactly that when deciding whether the phase it clears is the
    barrier it just satisfied. A tag-only comparison passes every oracle test.
  - **`BarrierResult` is one i64**: a count ≥ 0 is `Waiting(n)`, and three
    negative sentinels carry the unit variants. `reach_barrier` runs once per
    node per barrier, so a returned struct would charge the no-free global bump
    on every call. `Waiting(0)` is unrepresentable and cannot arise — the oracle
    answers `AllReached` whenever `reached >= total`. The hazard this creates is
    that **0 means "unknown run", not 0.0 progress**, and it is pinned.

  Also past the oracle: an emptied participant set satisfies **every** pending
  barrier (the empty set is a subset of anything, reachable through
  `remove_node`); `reach_barrier` does not clear the arrival set, so a second
  pass over one barrier name answers `AllReached` immediately; `force_barrier`
  is unconditional and revives a Completed run to Running; and
  `overall_progress` answers 1.0 for a run with no tasks, not 0.0.

  ⚠ `is_checkpointing` is **observably always false** — `checkpoint()` sets it,
  pushes, and clears it before returning, and every oracle method takes
  `&mut self`. The field and both writes are ported anyway: the oracle's own
  comment says it exists so barrier operations can be queued against it, which
  is a contract for a future concurrent caller rather than dead code. Recorded
  in `roadmap.md` section E.

- **`fleet/` exists** — `src/fleet/mod.cyr` (the group hub) and
  `src/fleet/cost_planning.cyr`, the first of the oracle's eleven `fleet`
  submodules. GPU pricing tables, crew cost estimation and budget-aware model
  selection: `agnosai_gpu_pricing_*`, `agnosai_estimate_crew_cost`,
  `agnosai_select_cheapest_model`, `agnosai_cost_tier_*`.

  `tests/fleet_cost_planning.tcyr` carries **48 assertions** — all ten oracle
  `#[test]` fns plus 38 the Rust tests never reach: ASCII case folding (drop it
  and all ten oracle tests still pass, since every one passes a lowercase name),
  the classification order that makes `gpt-4` win over `gpt-3.5`, the full tier
  mapping, the empty-identifier guard, a byte-exact `Display` (the oracle
  asserts only two `contains`), and the `llama-7b`/`mistral-7b` tie **resolved**
  rather than merely accepted — both classify open-small, so their costs are
  bit-identical and the sort decides.

- **`fleet/environment`** — container/VM detection and cgroup resource limits.
  **62 assertions**, all 8 oracle tests plus 54 past them.

  ⚠ **Two of the oracle's eight assert essentially nothing** —
  `detect_returns_valid_variant` checks only that the answer is one of five,
  and `resource_limits_runs_without_panic` asserts nothing at all. Both measure
  whatever machine the suite runs on, because the oracle opens `/proc/1/cgroup`
  and decides in one function. Applying standing rule 8, every decision here is
  split from its file read, so the pure halves take their input as a parameter
  and can be driven with inputs a real machine never produces. Three
  mutation-verified: testing `kubepods` after `docker` (a real k8s cgroup names
  both, so reversing misreports every node as a bare container), dropping the
  `1<<62` v1-memory sentinel (unlimited would read as a ~9 exabyte cap), and
  skipping digit validation (`str_to_int` answers 0 for garbage, so an
  unreadable file would report a **0-byte** memory limit).

- **`fleet/relay`** — inter-node messaging, ordered and deduplicated.
  **58 assertions**, all 9 oracle tests plus 49 past them.

  **A genuine wrapper over majra — but only because four upstream defects were
  fixed first.** The port plan predicted a near-1:1 mapping and the *names*
  matched; the semantics did not. Wrapping majra 2.5.3 would have shipped a
  `relay_receive` that was not reentrant (file-scope globals *and* no lock,
  against agnosai's 100-worker pool), a discarded `is_broadcast`, no
  sequence-gap detection, and an undocumented replay window. All four are fixed
  in **majra 2.6.0**; this module delegates and does not reimplement dedup,
  sequencing or fan-out.

  Two divergences remain, documented rather than hidden and owed to majra:
  **channel capacity is ignored** (majra hardcodes 256; the wrapper records
  what was asked so it is visible), and **the message timestamp is monotonic,
  not wall clock** — the oracle stamps `Utc::now()`, so a serialised message
  would carry a meaningless number across processes. Nothing serialises one
  today. Both asserted as they *behave*, so neither reads as accidental.

  Also pinned: the payload is carried **by reference**, not copied — a caller
  mutating it after sending changes what every receiver sees.

- **`fleet/discovery`** — pluggable node discovery. **29 assertions**, all 7
  oracle tests plus 22 past them. The `DiscoveryBackend` trait becomes the same
  function-pointer vtable `src/tools/native.cyr` uses for `dyn NativeTool`;
  dropping `async` costs nothing because both shipped backends are synchronous
  in fact. Mutation-verified: the oracle's `self.nodes.clone()` means a caller
  that pushes to the result must not affect the backend — returning the stored
  vec passes every oracle test, because none of them mutates the result.

  ⚠ `DnsDiscovery` returning empty is **the oracle's own stub**, not a port gap
  — `rust-old` says so in its doc comment, and `lib/net.cyr` has no SRV
  resolver either (verified, not assumed).

- **`fleet/placement`** — the five scheduling policies (GpuAffinity, Balanced,
  Locality, Cost, Manual) with `PlacementRequest` builders, `place` and
  `rank_nodes`. **58 assertions**, all 12 oracle tests plus 46 past them.

  Two behaviours the oracle's own tests structurally cannot reach:
  - **The sort must be STABLE.** Rust's `sort_by` is, so tied scores keep input
    order and `place()` returns the earliest — and ties are the common case
    (equal VRAM, equal capability fraction, equal resource total). `vec_sort_by`
    is introsort, stable only below 18 elements. The comparator falls back to
    the node's original index, making the order total and reproducing
    stable-descending at any size.
  - **`Balanced`'s index is PRE-filter.** `enumerate()` runs before `filter()`,
    so a disqualified node still consumes its position; filtering first would
    silently re-rank everything behind an offline node.

  ⚠ **The first stability test was worthless and mutation caught it.** 40
  all-tied nodes proved nothing — `vec_sort_by` opens with an already-ordered
  scan, so an all-equal input is already final and the sort never permutes; the
  mutant survived. Rewritten with two 20-element tie groups in an order that
  *forces* permutation, the mutant dies on two named assertions.

  Also pinned: a hardware requirement *replaces* the legacy GPU checks rather
  than adding to them (reads like a bug, is the documented precedence);
  `GpuAffinity` disqualifies a CPU node even when `required_gpu` is false; and
  `Manual` with no preferred node places nowhere rather than falling back.

- **`fleet/gpu`** — `ComputeScheduler` / `ComputeAllocation` and the legacy
  `GpuDevice` view: add_device, add_gpu, allocate, release, best_device,
  devices_of_type, total/available memory with an optional accelerator filter.
  **65 assertions**, all 15 oracle tests plus 50 past them.

  - **`max_by_key` returns the LAST maximum**, and both `allocate` and
    `best_device` pick that way — so with two devices holding identical free
    memory, which wins is decided by that rule. The oracle's fixtures are always
    24 GB + 80 GB, so no oracle test ever ties. Mutation-verified: turning the
    comparison from `>=` to `>` fails three named assertions.
  - **Allocating twice for one task id LEAKS the first allocation.**
    `allocations` is keyed by task id and `allocate` ends with an insert, which
    replaces; the first allocation's memory stays charged with no record, so
    `release` can never return it. A real oracle defect, reproduced deliberately
    and asserted (10 GB charged, one record, 5 GB returned).
  - **`AGNOSAI_ACCEL_ANY` is -1, not 0**, because 0 is `AGNOSAI_ACCEL_CPU` — the
    port's usual "0 means None" would silently mean "CPU only".
  - Cyrius has no type aliases, so the oracle's `GpuScheduler` / `GpuAllocation`
    / `vram_mb()` / `total_vram_mb` compat surface becomes delegating fns rather
    than being collapsed away, which would be a silent API narrowing.

  ⚠ The two `saturating_sub` guards are **unreachable** and kept for oracle
  shape — mutation-verified (deleting one leaves all 65 green), and recorded in
  `roadmap.md` section E rather than counted as tested.

- **`fleet/registry`** — the node vocabulary `placement`, `gpu`, `state` and
  `coordinator` all speak, so it lands before them. `NodeInfo` / `NodeStatus` /
  `NodeRegistry`: register, heartbeat, unregister, get, list, list_online,
  update_statuses, count, count_online, find_by_capability.

  `tests/fleet_registry.tcyr` carries **67 assertions** — all 20 oracle
  `#[test]` fns plus 47 the Rust tests never reach:

  - **The `update_statuses` arm order is observable.** The oracle sleeps past
    *both* thresholds, so its own test cannot tell whether Offline is checked
    first. It must be: the TTLs overlap by construction (90 s ≥ 30 s), so a
    Suspect-first implementation pins a long-dead node at Suspect forever.
  - **The sweep only ever demotes** — a second sweep must not revive an Offline
    node; only a heartbeat restores Online.
  - **A `Draining` node is not exempt** from going Offline. The oracle's loop has
    no status guard, and adding one is the obvious "improvement" that would be a
    divergence.
  - **`gpu_count` saturates at `u32::MAX`**, reproducing
    `u32::try_from(n).unwrap_or(u32::MAX)`. A truncating cast turns 2³² into 0
    and silently marks a huge node GPU-less.
  - **A capability listed twice yields one hit** — the oracle's `.any()`
    short-circuits, and a naive loop would double-count and inflate fan-out.
  - **`Suspect` is not `Online`** — it reads as a soft state and treating it as
    live is the natural mistake.

  Two clocks are carried deliberately, as the oracle does: `clock_epoch_secs()`
  for the serialised `last_heartbeat`, and `clock_now_ns()` for the
  `#[serde(skip)]` instants that are the **only** thing liveness measures. An
  NTP step must never resurrect an offline node or evict a healthy fleet.

  ⚠ Not mutex-guarded, matching the oracle (whose `&mut self` lets the borrow
  checker serialise). Cyrius has no borrow checker, so a caller sharing one
  registry across `run_pooled` workers must add the lock — exactly what
  `src/tools/registry.cyr` had to do. Nothing in `src/` shares one today.

  Three notes worth keeping:
  - `_agnosai_contains_ci` walks the haystack in place rather than lowercasing
    into a buffer. `lib/string.cyr`'s `str_lower_cstr` allocates per call inside
    a per-candidate loop, and the in-place scan copies nothing.
  - `AgnosaiCostTier` is deliberately **not** `AgnosaiModelTier` —
    `src/llm/router.cyr:24` already owns that name and the `AGNOSAI_TIER_`
    member prefix. Cyrius is **silent** on a duplicate enum; only
    `scripts/check-symbols.sh` caught it.
  - `agnosai_gpu_pricing_default` builds 1.20 / 0.35 / 0.80 by division rather
    than as literals, so the bit patterns match what Rust's literal parse
    produces.

### Removed

- **`probe_key_tmp.pem` — a live RSA private key committed at the repo root.**
  1,704 bytes, tracked, committed in `bb76e67 "errors and jwt work"`, and
  referenced by **nothing** in the tree.

  Verified unneeded three ways before removal: it is not the oracle's fixture
  (sha256 `36c3a7e2…` vs `a5cdfea6…`), its public half does not match the frozen
  vectors agnosai verifies against (`…AQEAlv/hFeMqWBO6…` vs
  `…AQEA5Wu/jjUwgB2e1/Bn…`), and both auth suites pass without it — **133 + 9
  assertions, 0 failures**.

  ⚠ **Deleting the file does not remove the key** — it stays reachable from
  `bb76e67`. **Maintainer decision: no history rewrite; accepted as is.** The key
  was generated locally for a probe and was never a credential for anything.
  Recorded so it is not re-raised as a finding later.

  agnosai never signs — it only verifies — and the one keypair any test needs is
  baked into `tests/server_auth_vectors.cyr` as frozen RS256 vectors precisely so
  that no key file has to exist. `.gitignore` now carries `*.pem` / `*.key` with
  that reasoning, since a PEM in this tree is always debris.

### Performance

- **`inference_queue_enqueue` 1,269 ns → 1,206 ns (−5.0%)** from deleting the
  redundant enqueue mutex (see **Changed**). That is the uncontended cost of one
  `mutex_lock`/`mutex_unlock` pair on this path — modest, and the honest figure:
  the lock was removed because its justification was gone, not because it was
  expensive. The dequeue rows are unmoved (88 → 91 ns at 2k, 89 → 88 ns at 16k),
  which is what should happen, since only `enqueue` ever took it.

  ⚠ **One row read +50.3% in that sweep and it is NOISE — recorded so it is not
  chased later.** `prompt_scan_clean_500b` printed 95,555 ns against a
  61,736–63,578 ns band across the five prior sweeps. Its three siblings
  (`clean_67b`, `clean_4k`, `suspicious_500b`) did not move, and nothing in this
  change touches `prompt_guard`. Re-measured three times immediately after:
  **62.65 / 62.85 / 62.22 µs**. A single row jumping with no sibling movement is
  the signature of a scheduling artifact, not a regression — the CSV keeps the
  outlier, this note explains it.

- **`inference_queue` background drain is no longer quadratic — the majra 2.6.2
  fix is confirmed in `bench-history.csv`.** 2026-08-12 is the first committed
  sweep carrying it, and the two rows converged as `benches/llm.bcyr` predicted:

  | depth | majra 2.6.1 | majra 2.6.2 | |
  |---|---|---|---|
  | 2,000 | 1,931 ns | **88 ns** | −95.4% |
  | 16,000 | 15,646 ns | **89 ns** | −99.4% |
  | slope across the 8× depth range | 8.1× | **1.01×** | flat |

  `pq_dequeue` popped with `vec_remove(tier, 0)`, shifting every survivor, so a
  full drain was **O(n²)**; it now keeps a read index per tier. This is the
  design case rather than a pathological one — the queue exists so background
  work is *allowed* to pile up behind interactive work.

  ⚠ **These are the only 2 of 199 rows that moved more than 25%**; the other 197
  stayed inside ±12% host noise, which is what makes the pair a signal rather
  than a sweep-wide shift. Measuring at **two** depths is what proved it: the
  absolutes move with run conditions (the same 2.6.1 build reads 1.9/15.6 µs in
  a sweep and 4.4/35.3 µs standalone, because a sweep warms the allocator), but
  the slope held at 8.1× vs 8.0×. A single-depth row would have been noise.

  ⚠ **Not comparable to the row above it in the CSV for the timer reason too**:
  from cyrius 6.5.19 `bench_batch_stop` subtracts one measured clock read per
  batch. At these iteration counts that is below the reported integer ns, so the
  series stays continuous — but it is a real change in what the number means.

### Changed

- **The defensive `[deps.sakshi]` pin is GONE, and the `inference_queue` mutex
  with it. Both were workarounds for upstream defects that are now fixed.**

  `[deps.majra]` 2.6.2 → **2.6.3** and `[deps.bote]` 3.3.0 → **3.3.1**.

  **The mutex.** `agnosai_inference_queue_enqueue` held a process-local lock
  across `queue_item_new` + `cpq_enqueue`, purely because majra builds the item
  with `fl_alloc` in the *caller*, outside `cpq_enqueue`'s own mutex, and
  `fl_alloc` was not thread-safe. cyrius 6.5.19 fixes that, so the lock is gone
  and `InferenceQueue` drops from 16 bytes to 8 (`AGN_IQ_MUTEX` deleted).
  ⚠ It was never a complete fix — it serialised agnosai against *agnosai* while
  any other thread calling `fl_alloc` still raced it. A consumer-side lock cannot
  close an allocator-side hazard, which is why re-adding one would not help if
  this ever recurs.

  **The sakshi pin — and the root cause was never where this manifest said.**
  The note claimed the overlaying sibling was bote's own `[deps.sakshi]`. It is
  not. The real chain is four links deep:

  ```
  agnosai -> bote -> [deps.libro] 2.8.4 -> [deps.patra] 1.12.12
          -> [deps.sakshi] 2.4.2
  ```

  ⚠ **`patra` is itself FOLDED INTO the cyrius stdlib** (`lib/patra.cyr` v1.12.12
  in the 6.5.19 snapshot) while carrying a git dep on a sakshi **eight patch
  releases behind** the sakshi that same snapshot ships — and `cyrius deps`
  overlays it, recursing through sibling manifests, on every `cyrius build`.

  bote 3.3.1 now pins sakshi forward at 2.4.10 to absorb that, which is what
  makes the pin removable here. **Verified by measurement, not reasoning**:
  removed, full three-step, then a build — `lib/sakshi.cyr` stays at 2.4.10 and
  `lib/` diffs clean. ⚠ The check that matters is **after a build**; the
  three-step alone was never the failure mode.

  ⚠ **A first attempt at this test was invalid and is recorded so it is not
  repeated.** Deleting the block with a naive string match cut from a *mention of
  `[deps.sakshi]` inside the majra comment*, silently removing `[deps.kavach]`,
  `[deps.ai-hwaccel]` and `[deps.tyche]` as well. The build failed with
  `undefined variable 'ACCEL_CPU'` — which reads like a sakshi-removal
  consequence and is nothing of the kind. Anchor on `^\[deps\.sakshi\]$`.

  ⚠ **agnosai's correctness here now depends on bote's pin**, which is acceptable
  only because the regression is *detected* rather than trusted:
  `scripts/check-clean.sh` diffs `lib/` against the toolchain snapshot and CI
  runs it. The standing instruction to re-derive the chain still holds — and to
  follow it **down**, not one level: the predecessor note was wrong twice for
  exactly that reason. **Sixteen repos in this workspace pin `[deps.sakshi]`,
  spanning 2.3.0 to 2.4.10.**

- **Toolchain pinned to cyrius 6.5.19** (was 6.5.18), which consumes **both**
  upstream defects filed from this tree. Full three-step
  (`deps --no-lock` → `lib sync --full` → `deps --lock`), `lib/` diffed against
  the 6.5.19 snapshot **after the sync and again after a build** — 107 files,
  zero drift. Full gate on the bumped tree: **97 suites, 0 failures.**

  - **`fl_alloc` is thread-safe** (`lib/freelist.cyr`). Filed 2026-08-10 after
    majra's `test_relay_receive_is_reentrant` failed intermittently. The filing
    described **one** race — two threads popping the same block off
    `_fl_heads[cls]`; upstream found **five**, and locked all of them behind a
    process-wide CAS spinlock: `fl_init`'s check-then-set, the pop, the push,
    the arena bump, and an arena refill whose `mmap` left a ~2 µs unlocked
    window that could return a block **running off the end of its mapping**.
    The large (>4096) path still takes no lock; it touches no shared state.

    ⚠ **This makes `src/llm/inference_queue.cyr`'s enqueue mutex redundant.** It
    was added solely because `queue_item_new` builds the item with `fl_alloc` in
    the *caller*, outside `cpq_enqueue`'s own mutex. It never closed the hazard
    — it serialised agnosai against itself while any other thread in the process
    calling `fl_alloc` still raced it, which is why the fix had to be upstream.
    The lock is **left in place pending a deliberate removal** and both the
    module header and the call site now say so; nothing below it needs it
    (`alloc` has had its own spinlock at `lib/alloc.cyr:28` throughout).

  - **The benchmark timer floor is measured, not declared** (`lib/bench.cyr`).
    Filed 2026-08-11 after decomposing two rows as "one clock plus the work" and
    getting the split backwards: at the documented `~120 ns` the clock is 6 % of
    a 2.045 µs row; at the real figure it is 64 %. `bench_clock_overhead_ns()`
    now calibrates one clock read at first use and every reporting path
    subtracts it.

    ⚠ **A corrected constant would still have been wrong**, which the filing did
    not anticipate. One clock read costs ~1,320–1,720 ns here, ~3,550 ns on
    aarch64 Linux, ~15–32 ns on macOS arm64 and ~64–68 ns on macOS x86_64 — a
    **230× spread** across the four release-gate hosts.

    ⚠ **And the cause the filing named is wrong on this host.** It blamed
    cyrius's raw `syscall(228)` versus libc's vDSO. Measured upstream, the vDSO
    path costs the same (2,456 ns vs 2,277 ns paired), because this box's
    `current_clocksource` is **hpet** — the kernel rejected the TSC at boot, and
    HPET has no userspace fast path, so the vDSO falls back to the syscall. The
    cost belongs to the clocksource, not the syscall choice. Corrected in place
    in `benches/fleet.bcyr`, `benches/harness.bcyr` and `benches/learning.bcyr`.

    ⚠ **The floor also moves between reboots** (the TSC-watchdog trip is
    per-boot; this machine has recorded ~400 ns and ~1,700 ns), so cross-run
    comparison of micro rows is only sound within one boot.

  ✅ **`bench-history.csv` is unaffected and the series stays continuous.**
  6.5.19's other half re-sized `bench_run`, which used to wrap a clock pair
  around every iteration and floored 57 of 79 rows in *cyrius's own* history at
  ~2 clock reads. **All ten agnosai `.bcyr` files use
  `bench_batch_start`/`bench_batch_stop` and none calls `bench_run`**, so that
  inflation never entered this tree's numbers. What did change is that
  `bench_batch_stop` now subtracts one clock read **per batch**, which at these
  iteration counts is below the reported integer ns.

  ⚠ **The 6.5.18 snapshot under `~/.cyrius/versions/` was overwritten in place
  with 6.5.19 content**, so `check-clean` failed against it before the bump and
  a 6.5.18-vs-6.5.19 `lib/` diff reads empty. Neither is drift in this tree; the
  gate passes against the 6.5.19 snapshot.

  `[deps.sakshi]` stays pinned at 2.4.10 and still matches the snapshot, so the
  documented flap does not return. Re-verified the sibling that makes that pin
  load-bearing rather than trusting the note: **bote still declares
  `[deps.sakshi]`**, at 2.4.8 in its working tree and 2.4.7 at tag 3.3.0.

- **The cleanliness gate and CI now cover `benches/` and `examples/`.** `scripts/check-clean.sh`
  swept `src/` and `tests/` and never `benches/`, and CI ran `check-clean.sh`,
  the tests and `cyrius coverage` but never `cyrius bench`. So **nothing in the
  pipeline compiled a `.bcyr`**, which is how three of them rotted for three days
  (see Added). `.bcyr` and `examples/*.cyr` now join the fmt loop (207 → 219
  files), the lint loop (111 → 122) and — for examples — the doc loop
  (111 → 112). CI gains **`Examples`**, **`Fuzz harnesses`** and **`Benchmarks`**
  steps, and `check-clean.sh` gains a **`doctest`** pass over every file carrying
  a `# >>>` block.

  ⚠ **Four `cyrius` gates existed and exactly one was in CI.** All five are now:

  | gate | what it found the day it was first run |
  |---|---|
  | `cyrius bench` | 3 of 6 `.bcyr` did not compile; 50 of 79 shapes dead |
  | `cyrius fuzz` | the only harness was a scaffold whose `fuzz_main` ignored its input — passing since 2026-07-28 |
  | `cyrius doctest` | never run; the tree had no doctest, and a note claimed the runner did not exist |
  | `cyrius coverage` | already gated, and clean — 1561/1561 |

  None of them was silent — each exits non-zero. They were never invoked, and two
  of the three were claimed absent without running bare `cyrius`.

  ⚠ **`examples/` was discovered by nothing at all** — not `cyrius tests`, not
  `cyrius bench`, not the `[build].entry`. The first file added to it would have
  rotted the same way, silently, which is why the gate went in alongside it
  rather than after it.

  ⚠ That step is there for the **compile**, not the numbers. CI timings are too
  noisy to gate on and `bench-history.csv` is recorded from a quiet machine —
  gating on a CI benchmark number would produce exactly the flaky red that
  teaches people to ignore a gate.

- **`[deps.kavach]` 3.11.8 → 3.11.9 and `[deps.majra]` 2.6.0 → 2.6.1.** Full
  three-step, with `lib/` diffed against the 6.5.18 snapshot **after the sync and
  again after a build** — zero content drift. `lib/kavach.cyr` reports 3.11.9 and
  `lib/majra.cyr` 2.6.1. Full gate on the bumped tree before any new work: **93
  suites, 0 failures.**

  majra 2.6.1 fixes a concurrent-`relay_receive` allocator race and drops its
  `[deps.sakshi]`; kavach 3.11.9 is the WASM backend `src/sandbox/wasm.cyr` runs
  on ([ADR 019](docs/adr/019-wasm-tools-spawn-wasmtime-directly.md)).

- **The 35 `duplicate fn` warnings on every build were audited rather than
  tolerated. Two of the four collisions are real; two of my own first readings
  were WRONG and are recorded here so they are not re-derived.**

  Cyrius has one flat namespace and last-definition-wins, and the rule that makes
  this matter was **measured**: a caller parsed *before* the redefinition still
  binds to the later body (`early_caller()` returns 22, not 11). So a duplicate is
  never "harmless because our copy comes first".

  ⚠ **What is NOT wrong, contrary to a first reading:**

  - **`err_io` (kavach ∩ sigil) does not diverge.** The claim that kavach's body
    won and gave sigil's callers its `CRYPTO` kind came from comparing the two
    error tables with the prefixes stripped — and **sigil has two unrelated error
    enums**. `err_io` uses the *syscall*-error one (`lib/sigil.cyr:26-33`, which
    switches from `SIGIL_ERR_` to `SYSE_` mid-enum); the `CRYPTO=8` came from the
    *verification*-error one at `:4157`. Against kavach's table the syscall enum is
    **identical on all eight shared kinds**. Confirmed at runtime with both bundles
    linked: `err_io(5, "probe")` → kind 8, errno 5; `err_unknown` → 7.
  - **`_sub_new` (majra ∩ libro) is not miscompiling here.** A probe reading the
    subscriber struct directly shows `sub + 8 == 0` — **majra's** body ran, not
    libro's — and `pubsub_publish` delivers correctly. agnosai is doubly safe
    anyway: `src/orchestrator/pubsub.cyr` calls no majra `pubsub_*`.

  ⚠ **The mistake behind both:** the warning's `file:line` was treated as the
  location of the later definition. It is not usable — `lib/kavach.cyr:11512` and
  `:11588` name a file of **11,321 lines**, so the offsets are into the
  preprocessed stream. Filed to cyrius as
  `2026-08-10-duplicate-fn-warning-file-attribution-is-wrong.md`, with the
  suggestion that it report *both* definitions. **Determine the winner by running
  the symbol, not by reading the warning.**

  What survives as real, both latent rather than live:

  | collision | assessment |
  |---|---|
  | `attestation_result_new` (kavach ∩ sigil) | Two unrelated functions sharing a name — 48 bytes / 7 fields / `alloc` against 16 bytes / 2 fields / `fl_alloc`. Inert only because neither library calls it. Needs a rename regardless; there is no shared implementation to agree on. |
  | `_sub_new` (majra ∩ libro) | Correct here **by accident of include order**, and both directions corrupt: libro winning makes majra's `pubsub_publish` `fncall1` a vec; majra winning makes libro's `stream_subscribe` write 8 bytes past a 16-byte block. `bote` links both and calls `pubsub_publish`, so it should run the probe in its own build rather than inherit this result. |

  Also: `path_exists` (ai-hwaccel ∩ kavach) differs only in `file_exists` vs
  `sys_access(F_OK)` — near-identical, though `access` resolves against the real
  uid. And `SpawnedProcess_pid` is **kavach against itself**, two `struct
  SpawnedProcess` declarations of 24 and 40 bytes whose only shared accessor sits
  at offset 0 in both — benign by luck, and one field reorder from not being.

  The `fl_alloc` filing also closed its own open question: **`alloc` does not
  share the hazard** — `lib/alloc.cyr:28` documents and implements a process-wide
  CAS spinlock. Two allocators side by side in one stdlib with opposite threading
  contracts, and only one of them says so.

- **The `[deps.sakshi]` note named the wrong sibling, and the pin is still
  load-bearing.** It said majra 2.6.0 at 2.4.8 was what forced the defensive pin.
  majra 2.6.1 dropped its `[deps.sakshi]`, and sigil dropped its at 3.12.7 — but
  **bote 3.3.0 still declares sakshi 2.4.7** (`git show 3.3.0:cyrius.cyml`; its
  working tree, which `path = "../bote"` makes the live resolution, says 2.4.8).
  Either way it is behind the snapshot's 2.4.10, so removing the pin still
  silently downgrades `lib/sakshi.cyr`. The note now names bote and tells the
  next reader to re-derive it with
  `grep -l '^\[deps\.sakshi\]' ../*/cyrius.cyml` rather than trust the line —
  it has been wrong twice.

- **`[deps.sakshi]` 2.4.8 → 2.4.10, and the JSON formatter consumes it.** The
  gap this port filed upstream — `sakshi_log_kv` flattening `key=val` into the
  message before the emit hook could see it — is fixed in sakshi 2.4.10, shipped
  in the released cyrius 6.5.16, and now consumed. `./build/agnosai` emits
  `"fields":{"message":"LLM client configured …","hoosh_url":"http://…"}` where
  it previously emitted one flat string, which is what the Rust oracle's
  subscriber produces.

  ⚠ **Consuming it was mandatory, not optional**, and this is the change that
  closes the long-running `lib/sakshi.cyr` snapshot mismatch. On 2.4.10 a hook
  that ignores the new sixth argument does not merely fail to structure the
  fields, it **drops them entirely** — verified against the real binary in both
  directions. agnosai's own `[deps.sakshi]` pin wins over sigil's and majra's,
  which are still on 2.4.8; that is exactly what the defensive pin was written
  for.

  ⚠ Field values are emitted as JSON **strings**. sakshi carries every value as
  bytes with no type tag, so an integer field renders quoted where `tracing`
  renders it bare. Sniffing digits would mis-type an id like `007`, so the
  divergence is stated rather than guessed at.

  The hook is now tested **end to end** through `sys_pipe` + `sys_dup2` — fd 2
  redirected into a pipe, sakshi emitting through the installed hook, bytes read
  back. That is the only thing that catches a hook ignoring the fields block: it
  passes every formatter assertion and drops every field in production.

- **Toolchain pinned to cyrius 6.5.16** (was 6.5.14, via 6.5.15). Three-step, `lib/` diffed
  after the sync AND after a build: 107 files synced, `lib/unicode/` untouched
  as `lib sync --full` skips it, and no drift across an implicit re-resolve.
  The 6.5.16 step moved `sakshi`, `sys` and two `syscalls_*` files.

  ⚠ **`[deps.sakshi]`'s 2.4.8 tag is now the thing lagging.** The 6.5.16
  snapshot ships a newer sakshi and the tag pin overlays 2.4.8 back over it on
  every resolve — the same silent-downgrade mechanism that comment was written
  to guard against, now pointing the other way. Deliberate for the moment:
  2.4.9 changes what `sakshi_log_kv` hands an emit hook, and this tree's
  formatter is written against 2.4.8. The pin bump to 2.4.10 and the
  `_agnosai_telemetry_json_hook` update are **one change** — on 2.4.9+ a hook
  that ignores the new sixth argument silently drops every `key=value` from the
  JSON rather than merely failing to structure it.

- **`order`: the lexicographic `Str` comparator is now shared.**
  `orchestrator/scheduler` and `orchestrator/plan_cache` each carried a
  byte-identical copy; `fleet/federation` needed a third, which is CLAUDE.md's
  extraction trigger. Both copies are deleted and `agnosai_order_str_cmp` lives
  in `src/order.cyr`, which already precedes every caller in `src/main.cyr`'s
  single-pass include order. Behaviour is unchanged — `orch_scheduler` (77) and
  `orch_plan_cache` (40) stay green — and `tests/order.tcyr` gains **16
  assertions** for the contract itself: prefix ordering, no case folding
  (`"Z" < "a"`), unsigned bytes, and an embedded NUL that does not terminate
  the comparison.

  ⚠ Both original copies masked each byte with `& 255` "to force an unsigned
  compare". **The mask was dead code** — `load8` zero-extends by definition of
  the language (`movzx rax, byte [rcx]` on x86-64, `ldrb w0` on aarch64), which
  a mutation confirmed: dropping it left all 64 assertions green, including the
  0xFF one written specifically to catch a signed read. Removed, with the
  reason recorded in place rather than a comment claiming it was load-bearing.

- **`[deps.majra]` 2.5.3 → 2.6.0**, which fixes the four relay defects reported
  from here — chiefly that `relay_receive` was not reentrant. Verified after
  vendoring: the new API is present, the file-scope globals are gone from the
  bundle, and `lib/` still diffs clean against the pin after a build.

- **Toolchain pinned to cyrius 6.5.14**, which folds **sigil 3.12.6**. Three-step,
  `lib/` diffed after the sync AND after a build: 107 files, zero differences.

  **The serialising mutex in `src/server/auth.cyr` is DELETED** — verified
  across four sigil releases on the pinned-lane harness before removing it:

  | vendored sigil | valid verified / 2000 | forged accepted |
  |---|---|---|
  | 3.12.2 | 1 | **888 of 400,000** |
  | 3.12.3 | 1 | 0 |
  | 3.12.4 | 712 | 0 |
  | 3.12.5 | 2000 | 0 |

  `tests/server_auth_lane_race.tcyr` stays as the regression guard — it caught
  every stage, so it demonstrably detects a relapse.

  **6.5.14 is a floor, not a preference.** sigil 3.12.6 fixes an **RSA-PSS
  authentication bypass** (a forged PSS signature verifying — the same class
  agnosai reported against PKCS#1 v1.5) and declares `cyrius >= 6.5.14`.
  agnosai reaches PSS through TLS 1.3 CertificateVerify
  (`lib/tls_native_hs13.cyr:257,260`) on every outbound HTTPS call from
  `tools/agnos.cyr`, `tools/remote_registry.cyr` and `guarded_fetch.cyr`.

  ⚠ **The root cause was a cyrius tail-call bug, and this tree's own probe
  reported a false negative on it.** A `return f(pointer-into-this-frame, ...)`
  with **exactly six arguments** is TCO'd, and the epilogue frees the frame
  before jumping. cycc declines TCO above six args — which is why the same
  localisation worked in a 10-argument function and "broke the KATs" in a
  6-argument one. **Arity was the variable, not the buffer.** The probe written
  here to rule out callee-clobbers-caller called the callee and then did more
  work — not a tail call — so TCO never applied and it returned clean. Recorded
  as standing rule 12 in `state.md`; fixed in cyrius 6.5.14.

  Previously 6.5.13 (sigil 3.12.5), 6.5.12 (sigil 3.12.4).

  ⚠ **The serialising mutex in `src/server/auth.cyr` STAYS.** Re-running the
  staging that sigil 3.12.4 asks consumers to run — mutex removed, two threads
  pinned to one lane, 2,000 verifications each:

  | sigil | valid verified (of 2,000) | forged accepted |
  |---|---|---|
  | 3.12.2 | 1 | **888** |
  | 3.12.3 | 1 | 0 |
  | **3.12.4** | **712** | 0 |

  The **bypass is closed** — forged accepts are 0 from 3.12.3 onward. The
  fail-closed race is 99.95% → 64%, not gone. Remaining shared state is
  `bn_mod` (`sigil/src/bignum.cyr:313-317`), whose `_bn_modrem`/`_bn_modn1` are
  still lane-banked; `bn_mont_modexp_pub` calls it for the R² setup, so the
  public verify path still reaches a lane. Reported upstream with the caveat
  that the CRT sign path scrubs that lane at `rsa.cyr:601`, so localising it
  must zero the local before return.

  Two traps this bump hit, both now in `state.md`: `lib sync --full` skips the
  hand-vendored `lib/unicode/` subtree, and `[deps.sigil]` tracks the **fold**
  rather than sigil's tags — bumping the tag alone changes nothing, and
  `cyrius deps` falls back to stale bytes *silently* when a tag does not resolve.

- **Toolchain pinned to cyrius 6.5.11** (was 6.5.10), by the full three-step
  (`deps --no-lock` → `lib sync --full` → `deps --lock`) with `lib/` **diffed**
  against the snapshot rather than trusting a green sync: 100 files synced, zero
  content differences, and the only `Only in lib/` entries the six declared git
  deps. `[deps.sakshi]` needed no move — 2.4.8 is already what 6.5.11 folds.

  **The win agnosai actually collects is `lib/fs.cyr`'s bump-allocator leak.**
  `is_dir` and `dir_list` took their getdents scratch from `alloc()`, which the
  default bump allocator never returns. Both are stack locals now.
  `src/orchestrator/durable_state.cyr` calls `is_dir` at :219, :248 and :281 on
  the crew-persistence path, so this was a live leak here.

  ### Performance

  **Measured on this box, not quoted.** The 6.5.10 and 6.5.11 shapes of `is_dir`
  were compiled into **one binary** over the same fixture — the old Linux arm
  lifted verbatim from `git show 6.5.10:lib/fs.cyr` as `_is_dir_610` — so the
  comparison needs no cross-build baseline. Both arms agree on the answer
  (`is_dir("/tmp") == 1`), so the delta is allocation and nothing else:

  | | bytes per call |
  |---|---|
  | `is_dir`, 6.5.10 | **4,104** |
  | `is_dir`, 6.5.11 | **0** |

  That reproduces upstream's own figure exactly. Confirmed independently against
  the real path: 1,000 `is_dir` calls on 6.5.11 charge the global bump **0
  bytes**, and `agnosai_state_store_save` now costs **1,360 B/save** — the
  residual being the save itself, not scratch.

  `lib/thread.cyr`'s only change is gaining `include "lib/thread_macos.cyr"`.

  Two traps worth recording, both hit doing it:
  - `lib/unicode/` is hand-vendored and `cyrius lib sync` copies **only the top
    level**, so `unicode/_decode.cyr` was silently left at the old revision. The
    recursive snapshot check in `scripts/check-clean.sh` is the only gate that
    sees this.
  - The lockfile must be written **last**. Syncing that one file after
    `deps --lock` produced `FAIL: lib/unicode/_decode.cyr (hash mismatch)`.

  ⚠ **`~/.cyrius/versions/<pin>/lib` is not a trustworthy reference for a
  version.** The *6.5.10* snapshot directory was found holding **6.5.11**
  content — `fs.cyr` hashed identical to 6.5.11's and carried "v6.5.11" comments.
  The authoritative check is the cyrius repo's git tag
  (`git -C ~/Repos/cyrius show 6.5.10:lib/fs.cyr | sha256sum`), which is what
  established that agnosai's `lib/` had never drifted: both allegedly-differing
  files hashed byte-identical to tag 6.5.10.

### Fixed

- **`GET /api/v1/presets` never used the per-request arena — the dispatch arm was
  missing.** `agnosai_route_list_presets_a` has existed since M10, but
  `_agnosai_route_dispatch_inner` called only the global form, so all 18 preset
  documents were parsed onto the **no-free global bump on every request** and
  never reclaimed. `benches/definitions.bcyr` prices that parse at ~744 µs.

  It was the **only** id in that ladder with an `_a` form left unwired — every
  neighbour above and below carries the `if (a != 0)` arm. Exactly what CLAUDE.md's
  "thread the `_a` allocator variants" rule exists to prevent. Found by the M12
  bench audit, which had recorded it as an inherent limitation ("no arena arm
  exists, so only a global bench is possible") rather than a defect.

  ⚠ **The regression test took three attempts, and the first two were worthless.**
  Both arms return the same 18 presets and the same 200 —
  `agnosai_route_list_presets()` is just `_a(default_alloc())` — so asserting on
  the body passes whether or not the wire exists, which is what version one did.
  Version two asserted `arena_used > before` and also passed reverted, because
  the outer dispatch allocates **32 bytes** of its own path handling into the
  arena. Measured both ways:

  | | arena growth |
  |---|---|
  | wired | **30,464 bytes** |
  | unwired | **32 bytes** |

  The shipped assertion is a threshold at 8 KiB — ~250× above the noise, ~4× below
  the signal — and it is verified by reverting the fix and watching it fail. A
  regression test that cannot fail is worse than none, and two of these three
  could not.

- **The tree was silently building sakshi 2.4.7 while its pin shipped 2.4.8**,
  reverting the `i64::MIN` decimal fix (`n = 0 - n` is a no-op there). Fixed at
  the cause, in the manifests.

  `sakshi` is in `[deps].stdlib`, so `cyrius lib sync --full` provisioned 2.4.8
  correctly — and then **every `cyrius build` put 2.4.7 back**, because build
  performs an implicit resolve.

  The cause was *not* the siblings' vendored `lib/`, which is what
  `check-clean.sh`'s comment had claimed since 2026-08-05 and what the shipped
  gate therefore excused. sigil and bote declare **`[deps.sakshi]` in their own
  manifests** at an older tag, and `cyrius deps` overlays that transitive
  resolution on top of the snapshot. Proven by hashing every candidate source:
  the file on disk matched `~/.cyrius/deps/sakshi/2.4.7/dist/sakshi.cyr`, not
  any sibling's `lib/`.

  - `[deps.sakshi]` bumped to **2.4.8** in sigil, bote and majra.
  - agnosai now pins `[deps.sakshi] = "2.4.8"` itself, so a lagging sibling
    cannot reintroduce the downgrade. majra already carried this pin for the
    same reason; its manifest comment described the mechanism correctly.
  - **The `sakshi.cyr` allowance is removed from `scripts/check-clean.sh`** —
    its own stated exit condition ("delete when a plain `cyrius build` leaves
    sakshi.cyr matching the snapshot") is met. Mutation-verified: restoring
    2.4.7 makes the gate exit **1** on both `deps --verify` and the snapshot
    check, and pointing the new pin at 2.4.7 reproduces the downgrade and the
    shadow warning.

  ⚠ `deps --verify` **cannot** catch this class on its own — the lock is written
  *from disk*, so a downgraded file simply gets its downgraded hash recorded and
  the check reports success. The snapshot diff is the only gate that sees it.

  All seven siblings were bumped to the 6.5.10 toolchain and re-synced as part of
  this (sigil 6.5.3, bote 6.5.4, kavach 6.5.5, ai-hwaccel 6.5.2, majra 6.4.83,
  libro 6.4.83, tyche 6.2.11 → all 6.5.10), and all still pass: sigil 64, bote
  14, kavach 2, tyche 1, ai-hwaccel 12/13, majra 3/4. **majra needed three
  stdlib modules declared** — `dynlib`, `fdlopen`, `async` — which 6.5.10 no
  longer supplies transitively; its remaining failure is `test_live.tcyr`, which
  requires Redis on :6379 and PostgreSQL on :5432 (confirmed absent).
  ai-hwaccel's is a pre-existing count assertion against its own uncommitted
  `BACKEND_AGNOS_GPU` work, unrelated to the toolchain.

- **cyrius 6.5.8: the submitted-crew thread no longer leaks, and the coverage
  gate is trustworthy again. Both were agnosai filings.**

  - **`agnosai_orchestrator_submit_crew` is detached.** It called
    `thread_create` and never joined, stranding 2 MiB of stack plus 4 KiB of
    TLS per submitted crew — a consumer-side reaper would have bounded it but
    could not close it, because a thread cannot free the stack it is running
    on. `thread_create_detached` has the child unmap its own stack in the
    trampoline tail, with the TLS carved from the top of that same mapping so
    one `munmap` frees both, and no handle allocated at all. Upstream measured
    100 unjoined threads at **210,124,800 B of VA before and 0 after**.

    Pinned here by reading `/proc/self/statm` across 32 submitted crews and
    asserting under 16 MiB of retained address space — at the old cost that
    would be ~67 MiB. Reverting to `thread_create` fails it. **VA, not RSS**: a
    thread stack is mostly untouched, so RSS barely moves either way and only
    the address space shows the leak.

    **6.5.8 is the floor for `orchestrator`** — against 6.5.7 or earlier
    `thread_create_detached` does not exist and the module does not compile,
    which is the right failure mode for a change whose silent one is unbounded.

  - **`cyrius coverage` is authoritative again, so the local workaround is
    gone.** 6.5.8 fixed four defects in it, including the fixed 1 MiB corpus
    this tree crossed. Verified rather than assumed: at 1,082,744 bytes — 34 KB
    past the old cap — the tool now reports **1084/1084**, byte-for-byte what
    `scripts/check-coverage.sh` reported. **That script is removed** and the
    corpus-size warning is out of `check-clean.sh`; keeping a second
    implementation of a fixed tool is two things to maintain and two things to
    drift.

  - Also arriving with the fold and not separately verified here: the
    `thread_join` lost-wakeup deadlock (~1 join in 150k, permanent and silent —
    `orch_crew_runner`'s parallel and DAG modes join real threads), and all
    twelve `i64::MIN` decimal sites via `fmt.cyr` / `string.cyr` / `log.cyr`
    plus sakshi **2.4.8**'s `_sk_fmt_int`. agnosai has **no decimal formatter of
    its own** — checked, not assumed — so it inherits every one of them.
    `lib/sakshi.cyr` is byte-identical to the 6.5.8 snapshot.

  **The bump order is load-bearing and was not obvious.** `cyrius deps` writes
  the lockfile from what it just copied, and `cyrius lib sync --full` changes
  files afterwards, so the two gates contradict each other unless the lock is
  written last:

  ```sh
  cyrius deps --no-lock     # resolve git deps into lib/
  cyrius lib sync --full    # refresh the stdlib over them
  cyrius deps --lock        # record what is actually there
  ```

  Standing rule 2 said `deps` does not refresh the stdlib; what it did not say
  is that `--no-lock` / `--lock` are what let both gates pass at once.

- **M7 audit (2026-08-04): three live defects, found by adversarial review of a
  fully green suite.** 43 findings confirmed across the eight sandbox modules,
  each verified by applying the mutation and re-running the tests —
  [docs/development/m7-audit-2026-08-04.md](docs/development/m7-audit-2026-08-04.md).
  Coverage was 100% and 4,372 assertions passed throughout; reference coverage
  counts whether a symbol is *named* by a test, not whether an assertion would
  *fail* without it.

  Fixed here:

  - **`spawn`: the child leaked the descriptors it had just `dup2`'d.** `dup2`
    duplicates, it does not consume, so `irf`/`owf`/`ewf` stayed open alongside
    fds 0/1/2. The child held extra write-end copies of the output pipes, which
    a grandchild inherits — so the parent's read does not see EOF until every
    copy is gone, not when the direct child exits. Closed now, guarded so a
    descriptor that already *is* 0/1/2 is not closed after being dup2'd onto
    itself.
  - **`cx`: the guest's stdin was closed twice.** The runner closed it so the
    guest would see EOF, then `persistent_terminate` closed the same number
    again on the timeout path — and a second close of a recycled descriptor
    hits whatever the process opened in between. The runner now disowns the fd.
  - **`cx`: one descriptor leaked per successful run.** The guest's stdout fd is
    the runner's to close and nothing did.

  The leak now has a test that counts descriptors via `fcntl(F_GETFD)` across
  eight runs; reverting the `sys_close` fails it. **Its first version asserted
  nothing** — it used `dir_open`/`dir_read`, which do not exist, so the calls
  were dead-code-eliminated and the test reported green. That is the exact
  failure mode the audit exists to find, committed while fixing the audit.

- **M7 audit, second pass (2026-08-05): the remaining live defects. Nothing in
  the audit crashes or leaks any more.** Findings H1, H7 and M10.

  - **A tool that stops reading its stdin killed agnosai itself** (H1, H7).
    Writing to a pipe whose reader is gone raises `SIGPIPE`, and the default
    disposition terminates the *writer*. Measured at exit **141** (128+13) on
    three runs out of three, for a 320,000-byte payload to a script that exits
    early — with no assertion output at all, because a dead process prints no
    summary. Production was masked only incidentally: the one `SIG_IGN` in the
    process came from sandhi's serve loops, so nothing protected a test, a
    benchmark, or a consumer that never starts a server. The oracle cannot die
    here because Rust installs the same disposition at startup.

    `agnosai_spawn_capture_input` now calls `signal_ignore(SIGPIPE)` before the
    fork. Anything under ~64 KiB was never affected — the kernel accepts it
    into the pipe buffer whether or not the child reads it.

  - **`EPIPE` is now told apart from `EAGAIN`** (H1, H7). The write loop
    treated every negative return as "the child has not drained yet, come back",
    which is right for `EAGAIN` and wrong for `EPIPE`, where there is no next
    turn. It now closes the write end and records the failure. The three
    backends then split exactly as the oracle does: `python` returns
    `failed to write to python stdin` (`python.rs:92-95` propagates with `?`),
    while `process` and `oci` log a warning and keep the output
    (`process.rs:133-137`, `oci.rs:158-162`). New accessor
    `agnosai_spawn_stdin_write_failed`.

  - **The child gets the default `SIGPIPE` back before `execve`.** `SIG_IGN` is
    inherited across `execve` where a handler is not, so ignoring it in the
    parent would otherwise follow every sandboxed tool into its new image — a
    tool running `cmd | head` would see `write` return `EPIPE` where the same
    tool outside the sandbox dies on the signal. `std::process::Command`
    restores the default in the child for this reason, so the restore is parity,
    not taste. The stdlib has `signal_ignore` and no counterpart; filed upstream
    as `2026-08-05-syscalls-has-signal-ignore-but-no-way-back-to-sig-dfl.md`,
    with `_agnosai_signal_default` local until it lands.

  - **A spawn that could not set itself up stranded its descriptors** (M10).
    Every pre-fork `return _agnosai_spawn_failure()` left whatever had already
    opened behind — and the failures that reach it are `EMFILE`/`ENFILE`, so
    the failure that fires on a full table consumed six more and never gave them
    back. The audit measured no recovery at all: every later attempt then failed
    at the *first* pipe. The four pipes now live in one flat block and
    `_agnosai_spawn_setup_failure(pipes, n)` closes exactly the ones that
    opened. `AGNOSAI_SPAWN_FAIL_SETUP` previously appeared in no test in the
    tree; it has one now, under a lowered `RLIMIT_NOFILE`.

  All four fixes are mutation-verified, each killed by a named assertion —
  except the missing `SIG_IGN`, which is killed by the exit-141 crash that is
  the defect itself. Sandbox suites: spawn 144 → **158**, process 108 → **114**,
  python 61 → **66**, oci 100 → **106**, all green.

- **M7 audit, third pass (2026-08-05): the security controls nothing held in
  place.** Findings H2, H4, H5, H6, H9, H10, M6, M8, M9, M22. Each was a real
  control with no assertion that would fail if it were deleted; three were also
  live defects in their own right.

  - **`kavach_bridge` scanned the wrong number of bytes, in both directions**
    (H2). `gate_apply` measures with `strlen` and a `Str` is not a C string, so
    `scan_output` handed it a borrowed pointer: `"ok\0AKIA…"` was scanned as two
    bytes and **released a credential as PASS**, while ten borrowed clean bytes
    were BLOCKed because `strlen` ran off the end into the rest of the arena.
    The oracle has neither problem — it passes a `String`, which carries its
    length. The gate now gets a NUL-terminated copy with interior NULs mapped to
    the newline `_concat_with_nl` already uses as a separator, so the whole
    artifact is scanned. `config_agent_id` had the same borrowed-pointer defect
    and now uses `str_cstr`. Filed upstream against kavach for a
    length-carrying `gate_apply`.

  - **Both loader-injection filters were untestable, not merely untested** (H4,
    H6). No developer or CI environment carries an `LD_*` — this box has none
    across 42 variables — so deleting either filter left its suite green, and
    the same mutant with `LD_PRELOAD` exported failed. Both now take the
    inherited block as a parameter (`_agnosai_sanitized_envp_of`,
    `_agnosai_process_envp_from`) and are driven with all four vectors planted.
    `process` had also **re-implemented the filter** rather than calling the
    shared one; it now calls it, so a mutation in `spawn.cyr` fails the
    `process` suite.

  - **The `FD_CLOEXEC` test's premise was false and it cost 30 s a run** (H5).
    It drove `sleep 30 & echo started` and claimed the parent would block
    without the bit — but the grandchild also inherits fds 1 and 2, which cannot
    be CLOEXEC, so the unmodified tree already took the full sleep and both
    assertions held either way. The replacement closes the grandchild's stdio
    too, leaving the errno pipe as the only descriptor that could hold the spawn
    open: 16 ms with the bit, 4 s without. **The spawn suite is now 10 s instead
    of 32.6 s.**

  - **cx's network isolation was a headline claim with nothing behind it**
    (H10). Dropping `require_ns` — which is what makes kavach apply
    `NS_NETWORK|NS_USER` — left all 62 assertions passing. It is now asserted on
    the policy and through a guest reporting its own uid, which is 0 inside the
    new user namespace the network namespace rides on.

  - **cx's "skips are real" guard was hardcoded to one developer's home** (H9).
    `/home/macro/.cyrius/bin/cycc_cx` was the only such path in the tree, and CI
    runs as `/home/runner` — so the meta-test protecting the entire execution
    half asserted nothing anywhere else. Renaming a binary away used to drop 22
    assertions, including ADR-006's acceptance test, and still exit 0. The guard
    now resolves through the module's own search, and a counter insists the
    execution half ran when both binaries are present.

  - **`agnosai_cx_interpreter_path` could never return 0** (M22), so its
    documented contract, `agnosai_cx_run`'s guard and two skip guards were all
    dead — and a bare relative name reached `access()` and `execve()`, **both
    resolved against the process's current directory**. The audit planted an
    executable beside the process and watched it run as the tool interpreter. It
    now requires an absolute, executable path.

  - Three more assertions that could not fail: `kavach_bridge`'s `WARN -> PASS`
    collapse (M6, the QUARANTINE case only ever exercised the other arm),
    `policy_for_trust`'s `enabled`/`redact_secrets` (M8 — with `enabled` clear
    `gate_apply` short-circuits before scanning, making every pinned threshold
    inert), and `spawn`'s `LD_PRELOAD` check (M9), whose `|| contains "L="`
    disjunct could never be false because the script emits that literal
    unconditionally.

  Every fix is mutation-verified. Sandbox suites: kavach_bridge 93 → **106**,
  spawn 158 → **169**, process 114 → **123**, cx 63 → **74**.

- **M7 audit, fourth pass (2026-08-05): the fail-open divergences and the
  vacuous-assertion backlog. All 43 findings are now fixed.**

  - **A 0-second manager deadline was no deadline at all** (M17). The helper
    returned seconds and every caller multiplied by 1000, so
    `default_timeout_secs = 0` became `AGNOSAI_SPAWN_NO_TIMEOUT` and a
    `/bin/sleep 3` ran to completion reporting `timed_out == 0`. The oracle does
    the opposite — `tokio::time::timeout` on `Duration::ZERO` fires immediately.
    It now works in milliseconds so 0 resolves to 1 ms, and **the OCI arm floors
    its conversion back to seconds at 1**: rounding 1 ms down to 0 restored the
    identical fail-open a layer lower, which only a second test caught.
  - **`policy` accepted negative durations and sizes** (M1). The oracle's fields
    are `u64`/`usize`, so serde refuses the document; the port read them signed
    and `-1` reached kavach as `timeout_ms: -1000`, where a deadline is armed
    only `if (timeout_ms > 0)` — so a negative duration armed none. Both fields
    now fall back to the default, as a wrong *type* already did.
  - **The OCI deadline was the one config field nothing pinned** (H8): deleting
    it silently substituted `oci_config_new`'s hard-coded 60 s, invisible
    because `/bin/echo` was the only stand-in runtime in the suite and it never
    blocks. Also pinned now: the OCI result's `timed_out`, `stderr` and
    `exit_code` (M18), all three of which could be hardcoded; the spawn-failure
    guard, whose deletion segfaults (M19); and the image-rejection message,
    which now matches the oracle's `"invalid OCI image: "` prefix and **names
    the offending character** (L10).
  - **`kavach_bridge`'s exec-failure arm had no assertion at all** (M7) — every
    `load64(&e)` check in that file asserted `== 0`. `sandbox_exec` returns 0 on
    BLOCK, so the ordinary "a tool printed a credential" case lands there. The
    start arm now carries kavach's error code, matching `format!("...: {e}")`;
    the other two cannot, because `sandbox_create` and `sandbox_exec` signal
    failure by returning 0, and that is now written down rather than left as an
    unexplained fixed string.
  - **`-v` was checked for membership, not position** (M4). `docker` parses
    options only until the first non-option argument, so an image emitted ahead
    of the mount makes it container argv and the volume is silently never
    applied — hoisting the image passed 100/100. And the runtime's `envp` was
    unpinned in both directions (M5): the suite now re-execs itself with a
    planted variable to prove the runtime inherits the **unfiltered**
    environment, which is the module's explicit parity decision.
  - **`python` logged nothing at all** (L7) — no `sakshi` call anywhere in the
    module, on the highest-risk operation in the subsystem, against a
    `#[tracing::instrument]` in the oracle and `debug!` lines carried across in
    every sibling. It now logs the spawn and the timeout, reports a bad
    `work_dir` as a working-directory fault rather than a missing interpreter
    (L8), proves `work_dir` reaches the interpreter (M14), and proves the
    sanitized `envp` does too (M15) — the last by re-exec, after a comment
    claiming that was unobservable turned out to be wrong.
  - **cx's timeout arm discarded output with no comment** (M23), on a test whose
    payload produced none; a non-positive budget installed no deadline while
    `AGNOSAI_CX_TIMEOUT_MS` was documented as the default and applied nowhere;
    and `AGNOSAI_CX_MAX_BYTES` appeared in no test in the tree. All three fixed,
    with the budget resolution split into `_agnosai_cx_budget_ms` so it is
    assertable in microseconds rather than by waiting out a 30-second default.
  - Nine more assertions that could not fail: `process()`'s `min_isolation`
    (M2), the `_a` allocator arguments (M3, measured per call — one span across
    both was satisfied by whichever still threaded), the wire field order (L2),
    the memory-field type guard (L1), the underscore in the image-reference set
    (L3), the twelve "failed executions" that all succeeded (L4), the
    replace-vs-append env semantics (M11), the chdir-vs-exec message
    discrimination (M12), `set_args` (M13), the `parameters` object tag (M16),
    and a PATH assertion the shell was inventing for itself (L6).

  **Two fixes are honestly unverifiable and are labelled as such**: L5 (a
  `sakshi_warn` length one byte short of its message) and L9 (the manager's
  missing dispatch `debug!`) are log-only, nothing here captures sakshi output,
  and both mutants survive. 41 of 43 killed, not 43.

  **The transferable finding is that the recurring defect was an
  *unreachable* assertion, not a missing one.** Five fixes worked by splitting a
  function so the thing under test could be handed its input; two more by
  re-execing the suite through its own spawn primitive with a planted `envp`,
  which is the only way a process with no `setenv` can put a variable in its own
  environment.

  Suites: policy 90 → **102**, oci 100 → **118**, kavach_bridge 93 → **136**,
  spawn 144 → **169**, process 108 → **130**, python 61 → **76**, manager 69 →
  **90**, cx 62 → **87**. **627 → 727 assertions**, all green, coverage 100%.

- **state.md said the test corpus was at 77% of the coverage tool's 1 MiB limit.
  It is at 100.5%, and has been since this session's own test additions.** The
  figure was stale, the limit is real, and crossing it is silent.

  `cbt/quality.cyr:59` reads every `.tcyr` into a fixed **1,048,576-byte**
  buffer; `if (n > 0)` makes a truncated read and a refused read the same thing,
  and neither is reported.

  **`tests/` is 1,067,457 bytes — 18,881 past the buffer, and it is no longer
  hypothetical.** The tool now reports **99%**, naming `sandbox/python.cyr`
  (14/15) and `routes/approval.cyr` (2/3) as carrying unreferenced functions.
  **Every symbol in both is referenced by a test.** Nothing regressed; the
  corpus stopped fitting.

  **`scripts/check-coverage.sh` is new** — the same computation with no buffer:
  same denominator (`^fn` in `src/**/*.cyr`, `_`-prefixed excluded), same
  definition of covered, so the two agree whenever the tool can answer. It
  reports **1074/1074 (100%)** and is the gate while the corpus is over.
  `scripts/check-clean.sh` prints the overage as a WARN every run rather than
  failing, because failing would leave a gate permanently red with no action
  available — what must not happen is the overage going unmentioned, since then
  the tool's percentage looks like evidence.

  Splitting the corpus does not help: `dir_walk` is recursive, and moving suites
  to a sibling directory drops their references entirely, which makes the number
  worse. The only real fixes are upstream's or a corpus under 1 MiB.

  Measured by padding one *unrelated* suite (`tests/order.tcyr`): 1,057,884 bytes
  → 99%, 1,113,884 → 94%, 1,253,884 → 85% with eleven source files reading as
  entirely unreferenced. Every step exits 0 and the gate says OK. The first
  casualty is a fn in `src/server/routes/approval.cyr`, which has nothing to do
  with `order.tcyr` — **bytes added to one suite delete another suite's
  evidence**, because they compete for one buffer.

  Filed upstream against cyrius as
  `2026-08-05-coverage-corpus-is-a-fixed-1-mib-buffer-and-silently-under-reports-past-it.md`
  with the root cause, the size table and three ordered fixes. Nothing is worked
  around here; corpus size is gated by hand until it lands, and the port plan's
  *"1 MiB corpus cliff"* constraint (`cyrius-port-plan.md:264`, predicted
  0.7–1.2 MB) is the entry that called it.

### Added

- **MCP `prompts/list` and `prompts/get` — roadmap B1 complete.** A prompt is a
  stored agent definition, rendered by the crew's **own** system-prompt builder.
  [ADR 016](docs/adr/016-mcp-prompts-project-agent-personas.md).

  Named by **agent key** — the same identifier `POST /api/v1/agents/definitions`
  stores it under and `agnosai://agents/<name>` addresses, so one name spans all
  three surfaces. Each declares one **optional** `task` argument, appended as
  `\n\nTask: …` when supplied and non-empty; the persona is useful without one.

  **The text is `_agnosai_crew_build_system_prompt`, not a second template.**
  What a client previews through MCP is byte-for-byte what a crew sends the model
  for that agent, so the two cannot drift. The test asserts that identity
  directly rather than re-describing the format — swapping in any other renderer
  fails it.

  Message role is **`user`**: MCP's `PromptMessage` has no `system` role, so the
  persona is the opening user turn, matching the specification's own examples.

  `capabilities.prompts` is advertised **without `listChanged`**, on the same
  reasoning that kept `subscribe` off `resources` in ADR 015 — definitions change
  at any time via the REST route and nothing here can push a notification, so
  claiming it would strand a client waiting.

  No new state, no new lifecycle: prompts appear and vanish exactly as
  definitions do.

  **7 mutations applied, 7 caught**, two only after a fixture fix — the fixture
  built its definition with `agnosai_agent_new`, which defaults `name` to the
  key, so nothing could tell `agnosai_agent_key` from `agnosai_agent_name` and
  the "named by key" requirement was unpinned. `server_routes_mcp` 104 → **124**
  assertions.

  agnosai now answers **seven** MCP methods against the oracle's three; every
  addition is additive, which is the basis ADR 015 was accepted on.

- **MCP `resources/list` and `resources/read`** — roadmap B1, first half.
  Stored agent definitions are addressable as `agnosai://agents/<name>` and
  readable as the same JSON `GET /api/v1/agents/definitions` already serves.
  No new state, no new lifecycle: it is a re-projection of existing surface over
  a second protocol, which is what bounds it —
  [ADR 015](docs/adr/015-mcp-resources-project-agent-definitions.md).

  **This is a superset of the oracle**, which answers three methods and would
  return `-32601` for these, so it is a wire-visible divergence and carries an
  ADR per CLAUDE.md. The three ported handlers are untouched and their tests
  unchanged.

  `capabilities.resources` is advertised **without** `subscribe`. A capability
  key is a promise a client acts on, and nothing here can push a
  `notifications/resources/updated` — the MCP route is request/response with no
  channel to the client.

  server_routes_mcp **80 → 104 assertions**, four mutations killed. The
  URI-scheme check needed a second attempt: `file:///etc/passwd` is refused
  whether or not the prefix is actually compared, because its tail matches no
  agent — so the first version of that assertion passed against a defeated
  check. The discriminating case is a foreign URI whose **tail is a valid agent
  name**, which a defeated check serves to a caller that never addressed it.

  **B1's "subscriptions" is struck, not deferred.** `lib/bote-core.cyr` has no
  `resources/subscribe`; what it has is `req_is_notification` /
  `dispatcher_notifications`, which is JSON-RPC notification detection and a
  different thing. Prompts remain owed.

- **Checked and left alone: MCP notification handling.** agnosai answers a
  request with no `id`, which JSON-RPC 2.0 §4.1 says a server must not do. The
  oracle does the same — `req.id.clone().unwrap_or(Value::Null)` and an
  unconditional `Json<JsonRpcResponse>` (`mcp.rs:49-65`) — so this is a faithful
  port, and "fixing" it would be the divergence.

### Changed

- **Pinned kavach 3.11.7, and deleted the gate workaround it makes
  unnecessary.** `agnosai_kavach_scan_output` now hands the externalization gate
  the artifact's **true length** via `exec_result_set_stdout_n`, instead of a
  `cstring` the gate measured with `strlen`.

  The audit's H2 finding was fixed on this side by
  `_agnosai_kavach_gate_bytes` — a copy that rewrote interior NULs to newlines
  so the whole artifact was at least visible. kavach 3.11.7 (filed by agnosai,
  fixed upstream) carries lengths through `ExecResult`, so **the copy and the
  rewrite are both gone**: the gate sees every byte, and the artifact reaching
  it is no longer altered on the way. That second half is the part worth having
  — a consumer needing the gate to scan the exact bytes could not previously
  have both.

  The two audit assertions are unchanged and still discriminate: reverting to
  the pre-3.11.7 `ExecResult_set_stdout(r, str_data(output))` fails both, one
  for the under-scan direction and one for the over-read. Sandbox suites all
  green against 3.11.7; `src/` symbol count 1,656 → 1,655.

  **3.11.7 is the floor for `kavach_bridge`** — against 3.11.6 or earlier
  `exec_result_set_stdout_n` does not exist and the module does not compile,
  which is the right failure mode for a change whose silent one is a released
  secret.

### Performance

- **`rank_agents` no longer diverges from the oracle in complexity — and got 5x
  faster at 1000 agents.** The port hand-rolled an insertion sort where the
  oracle uses `sort_by` (pdqsort, O(n log n),
  `rust-old/src/orchestrator/scoring.rs:212`). Replaced with `agnosai_sort`
  (introsort) — the same call `fleet/placement.cyr:287` already makes for the
  same job.

  | agents | insertion sort | introsort | |
  |---|---|---|---|
  | 100 | 127.8 µs | **95.8 µs** | −25.0% |
  | 300 | 626.0 µs | **296.9 µs** | −52.6% |
  | 1000 | 5093.0 µs | **1021.0 µs** | **−80.0%** |

  Scaling per 10x the agents: **39.9x → 10.7x**. The quadratic term is gone, and
  the remaining growth is the linear scoring plus the sort's log factor.

  ⚠ **Behaviour-preserving, and the reason is `_agnosai_scored_cmp` being a TOTAL
  order** — index ascending breaks every score tie, so no two entries compare
  equal and an unstable sort cannot reorder them. The old comment argued the
  opposite way round, treating the insertion sort's stability as what made ties
  safe and the index tie-break as "belt-and-braces"; it is the tie-break that is
  load-bearing, and it always was.

  ⚠ **The guard had to be widened before the swap.** `tests/orch_scoring.tcyr`
  pinned ties with **6** identical agents — below `_vec_introsort`'s 16-element
  threshold, so it exercises introsort's own insertion path and would have passed
  even if the partitioning path reordered. Added a 40-agent case first;
  **109/109 identical either side of the change.**

  This was left as "fix or ADR" in bite 3 pending the benchmarks. With them it is
  plainly a fix: before the 100/300/1000 series, the only ranking bench used 16
  **identical** agents, so the insertion sort did zero shifts and the quadratic
  was invisible for two weeks.

- **`scheduler_load_dag` is 2.46x faster at 500 tasks — by deleting dead work,
  not by optimising anything.**

  | bench | before | after | |
  |---|---|---|---|
  | `scheduler_load_dag_linear_50` | 154.8 µs | **136.2 µs** | −12.0% |
  | `scheduler_load_dag_wide_100` | 561.1 µs | **485.4 µs** | −13.5% |
  | `scheduler_load_dag_linear_500` | 3129.0 µs | **1271.0 µs** | **−59.4%** |

  `agnosai_scheduler_load_dag` and `agnosai_scheduler_topological_sort` each
  insertion-sorted **every** DAG key before calling `agnosai_scheduler_kahn_sort`
  — and `kahn_sort` sorts the zero-in-degree seed itself
  (`scheduler.cyr:169`) and sorts each successor list inside its loop (`:180`).
  The caller's key order therefore could not reach the output: the seed is
  re-sorted regardless and `in_degree` is a map. `load_dag` was worse still — it
  **copied** all the keys into a second vec first, paying an allocation and a
  full O(n²) insertion sort per load to produce an ordering the callee discarded.
  `map_keys` order is pseudo-random, so that is ~n²/4 ≈ 62,500 string comparisons
  at n=500.

  ⚠ **This is NOT a parity divergence, and an audit agent reported it as one.**
  The output is byte-identical with and without the pre-sort — which is the
  entire reason it could be deleted rather than needing an ADR. The claim that it
  diverged from the oracle came from reading `load_dag` without reading
  `kahn_sort`.

  ⚠ **Proven, not reasoned.** `tests/orch_scheduler.tcyr` gained a
  twelve-roots-inserted-in-reverse case plus a mixed roots/dependents case
  *before* the deletion — the only shapes where an unsorted seed could surface —
  and the suite reports **86/86 identical either side of the change**. The
  saving growing with n is the O(n²) term leaving, which is why the benchmarks
  had to exist first: this was deliberately left unfixed in bite 3 until they did.

- **cyrius 6.5.10 lands the `alloc_via` fix agnosai filed — every threaded
  route gained 5–13%.** `alloc_via` **15.1 → 11.1 ns (−26%)**, `reset_via`
  16 → 12.

  Both suggestions from the filing shipped: `alloc_via` now inlines its two
  accessor loads (`fncall2(load64(a), load64(a+32), size)`), and
  `arena_allocator` registers `&arena_alloc` / `&arena_reset` directly instead
  of the `_arena_*` trampolines. What remains — `alloc_via`'s own frame plus the
  `fncall2` indirection — is inherent to a vtable: `arena_alloc` called directly
  still measures 6.2 ns.

  | route (arena arm) | 6.5.9 | 6.5.10 | |
  |---|---|---|---|
  | `mcp` | 2,867 | **2,486** | −13.3% |
  | `tools` | 1,647 | **1,431** | −13.1% |
  | `dashboard_crews` | 5,252 | **4,656** | −11.3% |
  | `definitions` | 1,938 | **1,742** | −10.1% |
  | `get_crew` | 2,479 | **2,245** | −9.4% |
  | `dashboard_agents` | 8,271 | **7,793** | −5.8% |
  | `approvals_post` | 2,334 | **2,198** | −5.8% |
  | `approvals` | 1,303 | **1,231** | −5.5% |

  **The global arm barely moved** (−0.3% to −4.4%, two rows up inside noise),
  which is the internal check on the claim: it allocates through `alloc()`
  directly and never touches the vtable, so only the arena arm should gain.

  `GET /api/v1/crews/{id}` is now **2,245 ns**, down from 3,178 when this
  sequence started — **−29%** across the key hoist, the value hoist and this.

- **`GET /api/v1/dashboard/crews` 6,881 → 5,217 ns (−24%) and 160 → 112
  allocations (−30%); `/dashboard/agents` −17%, `/mcp` −14%.** The wire
  *values* and the remaining serialiser keys are minted once, like the crew
  keys before them.

  The previous bite hoisted fifteen keys off `crew_state_to_value_a`. This one
  found the rest by measuring what an allocation actually costs and then
  counting them, rather than by grepping for literals.

  **`alloc_via` is 15.1 ns**, whatever it is handed — measured against a 16 ns
  `reset_via` control, ten allocations per iteration. `arena_alloc`'s fast path
  is about eight instructions, so that is the vtable call chain, not the bump.
  A `str_from_a(a, "completed")` is one of those plus a 16-byte header.

  **Counting them exactly**, with a counting allocator wrapped around the
  arena's vtable (`allocator_new(&counting_alloc, .., arena)`):

  | route | allocs before | after | bytes before | after |
  |---|---|---|---|---|
  | `GET /api/v1/dashboard/crews` | 160 | **112** | 4,160 | 3,392 |
  | `GET /api/v1/crews/{id}` | 62 | **59** | 2,112 | 2,064 |
  | `POST /mcp` `tools/list` | 45 | **40** | 1,596 | 1,516 |
  | `agnosai_route_resolve_a` alone | 3 | 3 | 48 | 48 |

  The dashboard's 48 are the whole story: it renders one object per registered
  crew, and six literals sat in that loop body — five keys and a status
  spelling — so the eight-crew fixture paid them eight times.

  Timings, `benches/server.bcyr` (arena arm / global arm):

  | route | before | after | |
  |---|---|---|---|
  | `dashboard_crews` | 6,881 / 8,963 | **5,217 / 6,828** | −24% / −24% |
  | `dashboard_agents` | 9,874 / 12,172 | **8,187 / 9,935** | −17% / −18% |
  | `mcp` | 3,345 / 4,923 | **2,870 / 4,276** | −14% / −13% |
  | `approvals_post` | 2,441 / 3,162 | **2,343 / 2,968** | −4% / −6% |
  | `get_crew` | 2,591 / 3,724 | **2,492 / 3,537** | −4% / −5% |
  | `definitions` | 1,972 / 2,751 | **1,933 / 2,660** | −2% / −3% |

  `route_resolve` (332 → 338–346 across four runs) and `tools_arena` (1,645 →
  1,636–1,648) touch none of this and moved inside their own run-to-run range.

  **The scope rule is "a literal on a per-item path", not "a literal".** The
  tree has 338 `str_from_a(a, "…")` sites over 237 distinct strings; this
  changes the ones a loop body or an unconditional envelope reaches, and leaves
  every error *message* inline — those are built at most once per request, and
  only for a request that already failed. `AGN_JK_ERROR` is the one failure-path
  exception, because the *key* is built for every error response and the
  unauthenticated 404 scan controls that rate.

  This is still not the constant-return hoist `state.md` declines: that was 149
  sites judged on bytes, for 2.5%, against 121 new symbols.

  Placement follows Cyrius's single-pass resolution: shared vocabulary lives in
  `core/json.cyr` and `server/routes/mod.cyr`, the first include of each tier,
  because a global defined beside its first user is invisible to any module
  included earlier. One literal gets exactly one `Str` — `crew.cyr`'s
  `AGN_JK_CSTATUS` was a second `"status"` and is folded into `AGN_JK_STATUS`,
  and `mcp.cyr`'s `AGN_JV_TEXT` aliases `AGN_JK_TEXT` rather than allocating a
  second `"text"`.

  Pinned by pointer identity, which is the only exact assertion available here —
  a re-minted spelling is 16 bytes and invisible to any byte bound.
  `_t_wire_values_are_hoisted` asserts the built value *is* the global, that
  five wire lookups leave `arena_used` at 0, and that the crew status, the task
  status and the agent state all answer the **same** `"completed"`. Six
  mutations, six caught — including two where the spelling stays correct and
  only the sharing breaks.

- **`GET /api/v1/crews/{id}` 3,178 → 2,634 ns (−17%), and every serialising
  route gained with it.** The JSON keys are minted once instead of per object.

  Decomposed first, because the route's cost was not where it looked:

  | part | ns | share |
  |---|---|---|
  | `agnosai_crew_state_to_value_a` | 1,661 | **52%** |
  | dispatch, response struct, auth | ~718 | 23% |
  | `agnosai_route_resolve_a` | 347 | 11% |
  | `agnosai_orchestrator_crew` (mutex + map_get) | 233 | 7% |
  | `agnosai_uuid_is_valid` | 219 | 7% |

  Inside the serialiser, `bayan_json_v_obj_set_a` is already minimal — a pair
  and a `vec_push`, no duplicate scan. What cost was the **key**: a
  `str_from_a("crew_id")` per call, ~32 ns and a 16-byte header each. Measured
  directly, four sets ran 430 ns with keys allocated against **314 hoisted**.

  Fifteen `AGN_JK_*` globals across `core/crew.cyr` and `core/task.cyr` now hold
  them, minted once at startup over static literals — process-lifetime, so safe
  to share across threads and arenas.
  `agnosai_crew_state_to_value_a` **1,661 → 1,216 ns (−27%)**.

  | route | before | after |
  |---|---|---|
  | `route_get_crew_arena` | 3,178 | **2,634** |
  | `route_get_crew_global` | 4,353 | **3,744** |
  | `route_dashboard_crews_arena` | 7,806 | **7,064** |
  | `route_definitions_arena` | 2,268 | **2,026** |

  ⚠ **This is not the constant-return hoist state.md declines.** That verdict
  covered 149 `return str_from("lit")` sites worth 2.5% of *bytes* against 121
  new symbols. These are fifteen keys on the hottest serialisers in the tree,
  measured in *time*, and every route that renders a crew, a task result or an
  agent pays them.

  Also in the same pass, both smaller and both without design questions:

  - **`_agnosai_uuid_scan` inlines its hex decode** — 32 calls to
    `_agnosai_hex_value` per validation, which had no other caller, so there is
    still one decoder. 219 → 192 ns.
  - **`_agnosai_map_to_value_a` stops once every live entry is out.** A slot
    walk is over *capacity*: a task result's metadata is one entry in a 16-slot
    map. Measured 195 → 191 ns — **within noise**, kept because it is correct
    and cannot hurt, not because it paid.

  **5 mutations applied, 5 caught — after the assertion was replaced.** A byte
  bound on arena usage (1,488 baseline, 1,600 threshold) caught a wholesale
  regression but **not a single re-allocated key**, which is 16 bytes and
  invisible at any usable threshold. The test now asserts **pointer identity** —
  the key the built object carries either *is* the global or it is not — which
  is exact, needs no calibration, and catches all three single-key mutations
  including the one nested inside `core/task.cyr`.

- **A crew run builds no event payloads when nobody is subscribed:
  `orch_crew_runner`'s own cost drops 9,360 → 6,088 bytes for a four-task crew
  (−35%).**

  `_agnosai_crew_emit` already returned early with no bus attached — but the
  caller had built the payload by then, and `agnosai_event_sender_send` with
  zero subscribers takes the mutex, iterates nothing and drops it. The two
  per-task payloads measure **344 + 304 = 648 bytes**, plus the crew-level
  `crew_started` and `crew_completed` objects.

  **Zero subscribers is the normal state**, not an edge case: a client attaches
  to `/api/v1/crews/{id}/stream` only when someone is watching that particular
  crew. `_agnosai_crew_wants_events` checks the receiver count once per run and
  guards all four construction sites.

  **Nothing observable changes.** With zero subscribers the payload is discarded
  either way. A subscriber attaching between the check and the send would miss
  that one event — which is already the contract: subscriptions are
  `chan_lossy`, a slow client is told it `Lagged` rather than served a backlog,
  and a client attaching mid-crew never receives what preceded it. The oracle
  also builds its `serde_json::json!` payload before checking `event_tx`, so
  this is a divergence in *when the work happens*, not in what a client sees.

  **The decomposition corrected an earlier figure of mine.** I had reported
  `crew_runner`'s residual as "~14 KB". Measuring the spec construction
  separately shows a four-task run at 13,832 B is **4,472 B of building the
  spec** — the caller's, and in production it comes from the request parse — and
  **9,360 B of `crew_runner`'s own**. The method that found it is the same one
  that corrected the "97% transient" claim: measure the parts, never subtract a
  serialised size from an allocation figure.

  3 mutations applied, 2 caught: breaking the guard loses events the existing
  subscriber test already requires, and removing the subscriber check fails the
  new allocation comparison. ⚠ **Un-guarding a single one of the four sites is
  not caught** — it moves both arms of the comparison toward each other without
  crossing. Catching it needs an absolute bound that would need re-calibrating
  whenever a payload changes shape; the test says so rather than leaving it as a
  gap to be discovered.

  `orch_crew_runner` 192 → **193** assertions.

  ⚠ **A benchmark row in this release is a slow run, not a regression.**
  `route_get_crew_arena` recorded 4,471 ns against a 3,126–3,883 band, and
  `route_get_crew_global` 5,474 against 4,231–5,244. Three consecutive
  re-measurements give **3,190 / 3,232 / 3,212** and **4,494 / 4,519 / 4,546** —
  both inside band. `conv_buffer_push_sliding_32`, in a module nothing here
  touches, moved with them, which is what prompted the re-run. The CSV row
  stands as measured because it is a log; this is the note that keeps it from
  being read as a regression.

- **B3: every bare `str_from` that a loop actually *repeats* is gone — 27 → 3,
  and the 3 are error `return`s that execute at most once.**

  The roadmap carried "~49 in-loop `str_from` hoists" from a hand count. A
  brace-tracking scan puts it at **46**, close enough that the estimate held —
  but it splits into two classes that want different treatment, and B2 is what
  separated them:

  | | count | what it costs |
  |---|---|---|
  | bare `str_from` in a loop | **27** | 16 B on the **no-free global bump**, per iteration, forever |
  | `str_from_a` in a loop | 19 | arena bytes, reclaimed at the next `reset_via` |

  Only the first is a leak. **24 of the 27 are fixed.** The other three sit
  inside loops but on `return` paths — a malformed OCI image name
  (`sandbox/oci.cyr:250-251`) and the DAG-deadlock error
  (`crew_runner.cyr:990`) — so they run once and hoisting them would move an
  allocation off the error path onto the hot one. Left deliberately; a scan for
  "`str_from` inside a loop" will keep finding them, and this is why.

  The 19 arena-backed ones are left too, and the distinction is now recorded
  rather than implied.

  Fixed, by where they were:

  - **`orchestrator/crew_runner` (11)** — three metadata keys per task *result*
    in `_agnosai_crew_build_profile`, two more in `_agnosai_crew_record_metrics`,
    the two event names in both the sequential loop and the concurrent wave, and
    one constant in the output-validation retry loop. A 4-task run drops
    **26,296 → 26,024 B** and an 8-task run **23,128 → 22,472** — ~82 B per
    task, scaling with task count.
  - **`server/routes/sse` (6)** — five on the lag/error arm, and one that
    matters much more: `_agnosai_sse_event_json(fa, e, str_from(""))` runs
    **once per streamed event** on a connection that may live for the whole
    crew. Every other allocation in that loop was already threaded onto the
    per-request arena and reset each iteration; this one was not, so it was the
    single site in the SSE path that compounded without bound.
  - **`tools/builtin/security_audit` (6)** — two keys per missing header, four
    per vulnerability.
  - **`sandbox/oci` (4)** — one flag per env pair and per volume in the
    argv builder.

  ⚠ **A single-iteration loop gets marginally worse** (+16 B), because the
  hoist pays the same allocation once whether the loop runs or not. Measured: a
  1-task crew run went 7,336 → 7,352. That is the correct trade — the cost is
  constant while the saving scales — but it is worth stating rather than
  implying the change is free everywhere.

- **The audit hashing path builds into a scratch arena: a four-task crew run
  drops 33,008 → 26,296 bytes (−20%), and a record costs 2,432 → 496 bytes on
  the global bump.**

  `_agnosai_audit_entry_hash` builds a whole JSON tree and serialises it purely
  to feed the digest, then discards both — and `agnosai_audit_record` calls it
  **twice** once the chain is at capacity (the new entry, plus re-anchoring the
  evicted one). `_agnosai_audit_sign` adds two builders and a 32-byte mac. None
  of it outlives the call; only the two hash strings do.

  Measured by decomposition — switching the audit chain and event bus off
  independently — the audit path was **~18 KB of a 33 KB four-task run**, the
  largest single consumer. Per crew run:

  | configuration | before | after |
  |---|---|---|
  | 4 tasks, audit + events | 33,008 | **26,296** |
  | 4 tasks, audit only | 32,544 | **25,832** |
  | 4 tasks, neither | 14,096 | 14,104 |
  | per record, global-bump cost | 2,432 | **496** |

  `audit_record` latency is unchanged at 28.3 µs — SHA-256 and HMAC dominate, so
  this was never about time.

  **Three design constraints, each found the hard way:**

  1. **The scratch is a parameter, not a thread-local.** It has to be
     per-thread — `_agnosai_crew_run_parallel` and `_agnosai_crew_run_dag`
     record from real OS threads, and a shared one would have them reset each
     other's buffers mid-hash, silently corrupting a tamper-evident log. But
     **`thread_local_get` faults on the main thread**, which has no TLS block
     unless `thread_local_init` is called, and calling that on a *worker* would
     orphan the block `thread_create` already installed — taking sandhi's
     request-arena slot with it. The first implementation used a thread-local
     and segfaulted immediately.

  2. **An arena costs its capacity permanently, so it must be created lazily.**
     The backing chunk comes from the no-free global bump. Creating one
     unconditionally per crew run made an *audit-less* run **4.2 KB worse**
     rather than better — the whole point is to amortise that cost over many
     entries, which cannot happen when there are none. It is now created only
     when a chain is attached, and runs without one are byte-for-byte unchanged
     (+8 B for the struct field).

  3. **1 KiB initial chunk, chosen by measurement**: 512 B → 26,344 · **1,024 B
     → 26,296** · 2,048 B → 27,304 · 4,096 B → 27,288. Below 1 KiB the arena
     grows an extra time and pays for a second chunk; above it the slack is dead
     weight.

  **Parallel and DAG modes keep the global-allocator fallback, and it turns out
  nothing is owed there.** They do not audit per task at all —
  `_agnosai_crew_audit_task` is called only from `_agnosai_crew_run_sequential`,
  and the oracle has exactly one `audit_record` call site, in its own
  `run_sequential` (`rust-old/src/orchestrator/crew_runner.rs:294`). So this is
  **parity, not a gap**, and there is no per-worker scratch to write. The
  `AGN_CR_SCRATCH`-stays-0 path is kept because the runner genuinely is shared
  across those workers — if they ever gain a per-task audit, a scratch read off
  it would be reset mid-hash by a sibling — but it is belt-and-braces today.

  **5 mutations applied, 5 caught — and a ratio threshold caught none of them.**
  Baseline 496 B; un-threading the mac buffer 528, the sign builder 584, the
  hash object 672, the serialise 1,144, against a global arm of 2,432. Every one
  of those clears `scratch * 2 < global` and even `* 4`. The bound is absolute
  at 528 with 32 bytes of headroom — all there is, since the smallest transient
  allocation on the path is that 32-byte mac — and the test says so, along with
  what to do when a future change trips it.

  Also asserted: **both chains still verify**. Hashing has to stay byte-identical
  through the scratch or every signature changes. And the signature is pinned at
  64 hex chars, because it is written into `str_builder_new_a`'s 64-byte inline
  buffer and **`str_builder_putc` has no `_a` form** — a longer digest would grow
  that builder on the global allocator regardless of the scratch.

  `orch_audit` 64 → **70** assertions.

### Performance

- **cyrius 6.5.9's three-state mutex: agnosai changed nothing and got 15–77%
  across the board.** This is the payoff on a number that was honestly
  attributed rather than worked around.

  `lib/sync.cyr`'s two-state mutex called `FUTEX_WAKE` on every release whether
  or not a waiter was parked — 394 ns per uncontended lock/unlock pair against
  46 ns for a scratch build with that line deleted. agnosai filed it on
  2026-07-29 with a repro, worked around nothing, and state.md recorded the
  prediction: *"a stdlib fix will show up as a straight improvement rather than
  needing any change on this side."* 6.5.9 took it to 48 ns.

  | benchmark | 6.5.8 | 6.5.9 |
  |---|---|---|
  | `tool_registry_get` | 512 ns | **115 ns** (−77%) |
  | `event_send_evicting` | 2,384 ns | **933 ns** (−61%) |
  | `event_round_trip_1_sub` | 1,982 ns | **908 ns** (−54%) |
  | `event_fanout_64_subs` | 100.6 µs | **53.4 µs** (−47%) |
  | `route_dashboard_crews_arena` | 10,950 ns | **6,887 ns** (−37%) |
  | `pubsub_publish_4_patterns` | 5,891 ns | **4,505 ns** (−24%) |
  | `plan_cache_get_hit` | 2,149 ns | **1,716 ns** (−20%) |

  Every route arm gained 15–37%. The registry lookup is a bare lock plus a hash,
  which is why it moved most.

  **Two conclusions in state.md were rewritten rather than left standing**: the
  `chan_try_send`/`chan_try_recv` figure of "about four mutex pairs" is no
  longer the right arithmetic and is flagged for re-measure, and the
  stalled-SSE-client reading — `event_send_evicting` against
  `event_round_trip_1_sub` — now holds with more margin but was mutex-dominated,
  so the old numbers should not be quoted.

### Changed

- **The arena spill wrapper is deleted; 6.5.9 ships it as a policy.** The
  hand-rolled `_agnosai_arena_or_global_alloc` / `_realloc` and the per-worker
  thread-local that cached them lasted exactly one day. agnosai filed
  `2026-08-06-arena-is-fixed-capacity-and-answers-0-so-unbounded-work-cannot-use-one.md`
  and 6.5.9 answered it with an exhaustion *policy* on the arena itself —
  `ARENA_FULL_SPILL` is what the wrapper did, and `ARENA_FULL_GROW` is the thing
  it could not do at all.

  `agnosai_spill_arena` is now three lines over `arena_allocator_set_on_full`,
  and `_agnosai_spill_for` sets the policy on sandhi's own request arena rather
  than wrapping it — idempotent, so the thread-local went too.

  **SPILL rather than GROW, deliberately.** A request arena wants a bounded
  per-worker footprint, and GROW retains its chunks across `arena_reset` — a
  worker would converge on its worst-ever request and hold that forever, across
  100 workers. The test now asserts `arena_capacity_total` is unchanged as well
  as that the arena filled, so **swapping the policy to GROW fails**, which is
  how that intent is pinned rather than merely commented.

  **6.5.9 is the floor for `server/serve.cyr`** — against 6.5.8
  `arena_allocator_set_on_full` does not exist and the module will not compile,
  which is the right failure mode for a change whose silent one is a segfault.

  3 mutations applied, 3 caught: no policy and explicit `ARENA_FULL_NULL` both
  reproduce the segfault; `ARENA_FULL_GROW` fails the footprint assertion.

### Fixed

- **The per-request arena was a cliff: threading the routes turned an unbounded
  leak into a segfault, and 200 registered crews were enough to hit it.**

  `arena_alloc` answers **0** when it cannot fit a request — it does not grow,
  and the stdlib ships no chunked or growable arena. That was harmless while
  every handler allocated from the no-free global bump: a large response was a
  leak, never a fault. Threading the routes onto a fixed 64 KiB per-request
  arena inverted it, because **nothing in `bayan_json_v_*`, `str_from_a` or
  `vec_push_a` checks its allocator's answer** — an exhausted arena hands out 0
  and the next store takes down the worker.

  **Measured, not theorised:** `GET /api/v1/dashboard/crews` with 200 crews
  registered segfaults through a 64 KiB arena (exit 139).
  `AGNOSAI_MAX_RETAINED_CREWS` is **1,000**, so this is an ordinary operating
  condition rather than an attack — and `GET /api/v1/crews/{id}` on a long crew
  and `GET /api/v1/tools` with many tools have the same shape.

  **This was introduced by the B2 threading work in this same release**, which is
  why it is filed as Fixed rather than as a property of the design: the routes
  went from "leaks a lot" to "crashes", and no measurement in the suite covered
  the ceiling because every fixture was small.

  `agnosai_spill_arena` serves from the arena and falls back to the global bump
  **per allocation**, so the ordinary case keeps the whole win and only the
  overflow costs what it used to. `agnosai_serve_handler` wraps sandhi's own
  request arena in one, cached per worker in a thread-local — sandhi still owns
  the arena and still resets it between requests; this changes only what happens
  at the ceiling.

  The trade is stated rather than hidden: bytes that spill are global-bump bytes
  and are not reclaimed. That is exactly the pre-threading cost, applied only to
  the overflow. **Degrade to the old behaviour rather than fault.**

  Pinned from both sides — the 200-crew render now completes with all 200
  objects present **and** the arena is asserted to have reached its ceiling, so
  the fallback is what carried it rather than a response that happened to fit.
  A second assertion holds the win: a response that fits still charges the
  global bump **nothing**. Both mutations — removing the fallback, or having it
  answer 0 — reproduce the segfault.

  `server_serve` 184 → **189** assertions.

### Security

- **`agnosai_audit_record` now owns the strings it keeps, closing the class
  rather than relying on every caller to.**

  The chain is process-lifetime and its callers are not: both of them
  (`_agnosai_orch_audit` and `crew_runner`'s equivalent) pass strings that
  originate in a request body, and the request path now parses onto a
  per-request arena. `event`, `level`, `message`, `provider` and `model` were
  **stored by reference** — so one caller passing a borrowed message would leave
  a **tamper-evident log holding reclaimed bytes**: entries that silently
  rewrite themselves, which is the worst place in this system for that.

  Nothing was broken — every caller happened to pass an owned Str, and the
  preceding bites made sure of it. But that is a property a log should not
  depend on one call site at a time, and it is the same reasoning
  `server_state`'s `definition_insert` already applies to its key. This is the
  item the previous entry named as a decision rather than an oversight; it is
  now done.

  `_agnosai_audit_str_own` passes 0 through, which is what `provider` and
  `model` carry today, so the guard is contract rather than defensive noise.

  ⚠ **`metadata` is the exception and remains stored by reference.** It is a
  bayan value tree and **bayan ships no deep-copy primitive**; a
  `build`-then-`parse` round trip would be one, at the cost of a full serialise
  per audit record on the crew hot path. The contract therefore stands for
  metadata alone — pass a tree that outlives the chain — and both callers build
  theirs with a bare `bayan_json_v_obj_new()`, which does. Stated in the
  function header so the narrowed contract is visible at the call site.

  **The clone is free at this scale**: `audit_record` 29,512 ns against a
  28,335–29,785 ns band over the preceding seven runs — the SHA-256 chain hash
  and signature dominate completely.

  **4 mutations applied, 4 caught**, three of them by segfault: storing the
  message, the event or the provider by reference fails the
  survive-the-scribble assertions, and removing the 0-guard crashes on the
  `provider`/`model` path both callers actually use. `orch_audit` 58 → **64**
  assertions.

  ⚠ My first version of the test asserted `agnosai_audit_verify(c) == 1`. **0 is
  valid** — the port's error convention, which every other assertion in that
  file already used.

### Performance

- **`POST /api/v1/a2a/receive` 17,192 → 15,624 bytes per request — the last
  write route, and the whole request path is now threaded.**

  `agnosai_a2a_req_from_value` borrowed the task id, the description and the
  four optional strings from the parse tree. The description reaches the
  registered `CrewState` as a task result's output and the task id reaches the
  audit trail through the crew name, so both outlive the request. Both are
  cloned now, and `create`-style two-allocator wiring applies: parse tree and
  response in the arena, the request itself from `default_alloc()`.

  `metadata` is the one member still pointing into the parse tree, and
  deliberately — it is re-serialised for the 64 KiB size check and never
  retained.

  As with `create_crew`, the reduction is small because the bulk is the crew
  *execution* this route kicks off, which is registry state and must not move.

  **A mutation corrected the source comment.** I had written that the crew-name
  builder "stays global" and pointed at `str_builder_new`; moving that to the
  arena changes nothing, because **`str_builder_build` is what allocates the
  finished Str** and a bare `build` yields a global one however the builder was
  created. Moving `str_builder_build` fails immediately. The comment now names
  the right call.

  3 mutations applied, 3 caught once aimed correctly.
  `server_routes_a2a` 72 → **83** assertions.

  New `_a` forms: `agnosai_a2a_req_new`, `agnosai_a2a_req_from_value`,
  `_agnosai_a2a_opt_str`, `_agnosai_a2a_failed`, `agnosai_route_a2a_receive`.

  **Every route in the table is now threaded**, and the residuals are all
  retained state rather than garbage: 0 B for the seven reads plus `/mcp` and
  `/approvals`, 392 B for a stored agent definition, and the crew-execution
  floors on `crews` and `a2a`.

  ⚠ **Left standing, and worth knowing:** `agnosai_audit_record` stores its
  `message` **without cloning**. Every caller now passes an owned Str, so
  nothing is wrong today — but that is a contract held by convention on a
  tamper-evident log, and a future caller passing a borrowed message
  reintroduces this whole class silently. Cloning inside `audit_record` would
  close it at the boundary.

- **The crew request deserialisers clone too — `POST /api/v1/crews` 21,184 →
  17,048 bytes per request (−19.5%). The thing that was actually at risk was the
  audit chain.**

  `agnosai_crew_req_from_value` borrowed the crew name and `process` from the
  parse tree, and `agnosai_task_req_from_value` borrowed each task's
  `description` and `expected_output`. Both now clone into the allocator they
  are given, and `create_crew_a` hands them `default_alloc()` while parsing and
  responding in the request arena — the same two-allocator shape as
  `create_definition`.

  **The reduction is modest and the floor is the point.** Most of what this
  route allocates is the crew *execution* — task results accumulated by
  `crew_runner` and held in the orchestrator registry — which is retained state,
  not per-request garbage. Only the parse tree and the response move. A route
  that suddenly dropped to near-zero here would mean retained crew state had
  been put in an arena, which is the corruption the boundary exists to prevent,
  so the assertion has a floor under it as well as a ceiling.

  **What retains the borrowed strings is not what I first assumed, and the
  mutations are what said so.** `CrewState` holds a *minted* UUID, a status,
  results and a profile — nothing from the request. Asserting on
  `agnosai_crew_state_crew_id` therefore survived every mutation. The real paths
  are two:

  - **the audit chain.** `_agnosai_orch_register` and `_agnosai_orch_finish`
    pass `agnosai_crew_name(spec)` as the audit **message**, and
    `agnosai_audit_record` stores it **without cloning** into a chain that lives
    in `AppState`. A tamper-evident log whose entries silently rewrite
    themselves is a worse outcome than most.
  - **the task result.** On the placeholder path `crew_runner` makes the task
    description the result's *output* (`crew_runner.cyr:460`) and audits it
    (`:772`), and results live in the registered `CrewState`.

  Asserting only on the crew name left `agnosai_task_req_from_value`'s clone
  unpinned; a mutation found that too. The test now checks both.

  New `_a` forms with bare wrappers: `agnosai_crew_req_new`,
  `agnosai_task_req_new`, `agnosai_crew_req_from_value`,
  `agnosai_task_req_from_value`, `agnosai_route_create_crew`,
  `_agnosai_crew_req_fields`, `_agnosai_task_req_fields`.

  **Also corrected: an earlier entry named `agnosai_crew_from_value` as the
  blocker.** That was inferred from the name. It deserialises a *persisted*
  crew, requires an `id` no request carries, and **has no caller in `src/` at
  all**. The live borrow was always in the request deserialisers.

  6 mutations applied, 5 caught; the sixth — forcing the nested agents onto the
  global allocator — survives correctly, since the agent is still *owned*, just
  from a different arena. `server_routes_crews` 124 → **141** assertions.

  ⚠ The first version of the new test **segfaulted**, and for its own reason
  rather than the code's: it read the response body after `reset_via`, and the
  response lives in the arena. Reading a threaded route's result after releasing
  its arena is a use-after-free on the *test's* side.

- **`agnosai_agent_from_value_a` clones what it keeps, which unblocks
  `POST /api/v1/agents/definitions`: 2,280 → 392 bytes per request (−83%).**

  The previous entry recorded why three write routes could not be threaded:
  `agnosai_agent_from_value` stored `bayan_json_v_str(key_v)` and its siblings
  **without cloning**, so a definition kept in `AppState.definitions` pointed
  into the parsed request body. That was invisible while the parse tree lived on
  the no-free global bump and would have become silent corruption on an arena.
  This is the fix that entry said was owed — a deep copy at the retention
  boundary, not a permanently un-threaded route.

  `agnosai_agent_from_value_a(al, v)` clones every Str into `al`, so the caller
  chooses the lifetime:

  - `agnosai_route_create_definition_a` passes **`default_alloc()`** for the
    definition while parsing the body and building the response in the request
    arena — two allocators in one function, deliberately;
  - anything that uses an agent and drops it can pass the arena instead.

  **392 B is the retained definition itself and cannot go lower**, which is the
  point: what remains on the global bump is exactly the object the route exists
  to keep. The assertion is bounded on both sides — a residual that stopped
  shrinking would mean the parse tree had drifted back to the global allocator.

  New `_a` forms, with bare wrappers: `agnosai_agent_new`,
  `agnosai_agent_from_value`, `agnosai_hw_requirement_new`,
  `agnosai_hw_requirement_from_value`. The last borrows nothing from its value —
  both of its string members become enum ints through
  `agnosai_accelerator_type_from_wire` — so its allocator covers only its own
  struct and vec.

  **The hazard test is inverted rather than deleted.** It used to assert a
  stored definition's name was *destroyed* after the arena it parsed from was
  released and scribbled over; it now asserts the name, key, role and goal all
  **survive** that. Its own comment predicted this ("if a future change
  deep-copies at the retention boundary, this test fails and should be replaced
  by its opposite"). A second test drives the whole route through an arena, to
  catch the wiring rather than the two calls in isolation.

  **4 mutations applied, 4 caught.** Re-borrowing the name or the key fails the
  survive-the-scribble assertions; allocating the definition from the request
  arena **segfaults** the suite (exit 139 — a failed suite with no `FAIL:` line,
  which is the documented crash signature); un-threading the parse fails the
  byte measurement in `server_serve`.

  `server_routes_agents` 45 → **53** assertions.

  Still owed, and now with a proven shape to follow. **The blocker is
  `agnosai_crew_req_from_value` and `agnosai_task_req_from_value`, not
  `agnosai_crew_from_value`** — an earlier draft of this entry named the last
  one from its name alone, and it has no caller in `src/` at all; it
  deserialises a *persisted* crew and requires an `id` the request shape never
  carries. The live borrow is the crew name and `process` in the request
  deserialiser, plus three Strs per task, all of which reach the orchestrator
  registry through `run_crew`. The agents inside a crew request are already
  owned, since they go through `agnosai_agent_from_value`.

- **The write routes: two reach zero, and three must not — the deserialisers
  borrow the parse tree into process-lifetime state.**

  A body-carrying route splits in two, and only one half is safely threadable.
  That distinction does not exist on the read path and is the substance of this
  bite.

  | route | B/req before | B/req after | µs before | µs after |
  |---|---|---|---|---|
  | `POST /mcp` | 3,224 | **0** | 5.618 | **3.808** (−32%) |
  | `POST /api/v1/approvals` | 1,352 | **0** | 3.731 | **2.983** (−20%) |

  Both retain nothing — `approvals` uses `task_id` as a `map_get` key and sends
  an int through a channel; `/mcp` executes and returns. So their parse trees go
  in the arena with the JSON-RPC envelope, every tool schema behind
  `tools/list`, the `deny_unknown_fields` allow-list, and the response.

  **The other three are deliberately un-threaded, and this is a correctness
  boundary rather than unfinished work.** `agnosai_agent_from_value` takes
  `bayan_json_v_str(key_v)` and its siblings **without cloning**, and the
  definition is then stored in `AppState.definitions`, which is
  process-lifetime. So a stored definition points *into the parsed request
  body*. Threading `bayan_json_v_parse_buf` there would have `reset_via` reclaim
  it at the end of the request, and the next request would be handed the same
  bytes — **a stored definition's name would silently become whatever the next
  caller posted.** `POST /api/v1/crews` and `POST /api/v1/a2a/receive` have the
  identical shape through `agnosai_crew_req_from_value` /
  `agnosai_task_req_from_value` and the orchestrator registry.

  **So the current code is correct only because the global allocator never
  frees**, and nothing stated that. `server_state`'s `definition_insert` had
  seen half of it — it `str_clone`s the *key*, with the comment "the caller's
  Str may be borrowed from a request buffer that does not outlive the handler" —
  and the value was left borrowed.

  It is pinned rather than described. `server_routes_agents` parses a definition
  through an arena, stores it, releases the arena, scribbles over it, and
  asserts the stored name is **destroyed**. Two mutations kill that test:
  making `from_value` `str_clone` (the name survives — which is what proves the
  test measures borrowing, and what the eventual fix will look like), and
  dropping the scribble (reset alone leaves the bytes intact). `server_serve`
  asserts the same boundary from the other side: `create_definition`'s arena arm
  must stay ≈ its global arm, so "thread them for consistency" fails in the
  suite rather than in production.

  **The fix is to deep-copy at the retention boundary**, not to leave the routes
  un-threaded forever. Owed under B2.

  New `_a` forms: `agnosai_route_submit_approval`, `agnosai_route_mcp`,
  `agnosai_route_fields`, `agnosai_route_field`, and the twelve internal
  `_agnosai_mcp_*` builders. The two MCP logging helpers keep their bare shape —
  they allocate nothing, so an allocator parameter would be noise on a hot guard.

  **9 mutations applied, 9 caught**, two of them only after the measurement was
  fixed:

  - `str_builder_add_str` un-threaded **passed**, because the mutation landed on
    the `delivered` branch and the fixture could not reach it: a delivered
    approval is *consumed*, so a 32-iteration loop takes that path once and the
    other 31 times falls through. `_agnosai_route_approval_message_a` is now
    measured directly on both branches.
  - Reverting `add_cstr_a` to `add_cstr` for `"Approved"` still passes, and
    correctly: at seventeen bytes the builder has not grown, and only the call
    that *triggers growth* allocates. Recorded in the source so the next reader
    does not mistake the threading there for decoration.

  `server_serve` 164 → **178** assertions; `server_routes_agents` 41 → **45**.
  Four new benchmark rows.

- **The tool vtable passes the allocator, and with it `/api/v1/tools` reaches
  zero: 1,720 → 0 bytes per request. Every GET read route now charges the
  global bump literally nothing.**

  `/api/v1/tools` sat at a floor through three bites — 7,384 → 1,696 → 320 →
  288 — and the floor was never the routes or registry tier.
  `agnosai_tool_schema` called the tool's own `schema_fp`, which rebuilds its
  schema and every parameter under it on each call, and the vtable passed no
  allocator. Nothing above it could reach that allocation.

  `schema_fp` is now `fn(a, ctx)` — **allocator first**, a change to the
  calling convention rather than an addition to it — and all **fourteen**
  implementors thread it. New `_a` forms with bare wrappers:
  `agnosai_param_schema_new`, `agnosai_tool_schema_new`,
  `agnosai_tool_schema_with_param`, `agnosai_tool_schema_param`,
  `agnosai_tool_schema`.

  **This is not an API break, and the reason is worth stating rather than
  assuming**: agnosai ships no `[lib]` block and no `dist/` bundle — `bins =
  ["agnosai"]` is the whole of it — so every implementor of the convention is
  in this tree and there is no external caller to break. Consumers reach tools
  over HTTP and MCP, not by linking and registering one.

  The final state of the read routes, bytes charged to the global bump per
  request when served through a per-request arena:

  | route | at the start of B2 | now |
  |---|---|---|
  | `GET /api/v1/dashboard/agents` | 4,368 | **0** |
  | `GET /api/v1/dashboard/crews` | 4,160 | **0** |
  | `GET /api/v1/crews/{id}` | 2,352 | **0** |
  | `GET /api/v1/tools` | 1,720 | **0** |
  | `GET /api/v1/approvals` | 576 | **0** |
  | `GET /api/v1/agents/definitions` | 384 | **0** |
  | `GET /api/v1/crews/{unknown}` (404) | 320 | **0** |

  `route_tools_arena` across the sequence: 3,074 → 2,344 → 2,259 → 2,229 →
  **2,168 ns**.

  **The assertion on this route was a bound with slack for three bites** — 2x,
  then 3x, then 5x — and the 2x version passed both the threaded and the
  un-threaded case, so it asserted nothing at all. It is `== 0` now, and there
  is no slack in that. **6 mutations applied, 6 caught.**

  `server_serve` 163 → **164** assertions.

  **This shipped a regression that per-suite verification missed, and the
  whole-tree run caught.** `callptr` does not check arity: two test tools in
  `tools_native.tcyr` still declared `fn(ctx)` and **compiled and passed
  71/71**, with `ctx` silently receiving the allocator and the real ctx
  dropped — invisible in a schema that ignores `ctx`. The detectable half was a
  direct call, `_agnosai_delegate_schema(0)`, which then passed 0 as the
  *allocator* into `alloc_via` and segfaulted: a failed suite with no `FAIL:`
  line. Mutation-testing against `server_serve` and `tools_native` showed both
  green. Recorded as standing rule 9 — when a signature reached through
  `callptr` changes, grep for every implementor and direct caller by name,
  because the build will not.

  Also moved on the same run, in modules nothing here touched:
  `qlearner_best_action_1000_state_actions` 1,253 → 1,390 ns and
  `select_nth_100k` 4.919 → 5.337 ms, both just above their recent bands, and
  three unrelated `*_global` route arms all at +8.5%. A uniform shift across
  untouched code reads as a slower run rather than a regression; recorded
  rather than explained away.

- **`agnosai_uuid_is_valid` — every GET read route except `/api/v1/tools` now
  costs the global bump literally nothing, and a rejected parse stops leaking.**

  With the router threaded, the last non-zero read route was `/api/v1/crews/{id}`
  at 16 bytes, and the residual was `agnosai_uuid_parse`: it allocates a 16-byte
  buffer for the decoded UUID, and **all five of its callers in `src/` threw
  that buffer away.** Every one was `if (agnosai_uuid_parse(x) == 0)` — a
  validity test that never wanted the bytes (`routes/crews.cyr` ×2,
  `routes/approval.cyr`, `serve.cyr`). `agnosai_uuid_is_valid` is that test
  without the buffer.

  | route | B/req before | B/req after |
  |---|---|---|
  | `GET /api/v1/crews/{id}` | 2,352 | **0** |
  | `GET /api/v1/crews/{unknown}` (404) | 320 | **0** |

  Both arms, because both validate the same id — and the 404 is the
  unauthenticated one.

  **The rejection path was also leaking, and that half is security-relevant.**
  `agnosai_uuid_parse` allocated its buffer *before* validating and returned 0
  from inside the loop, so every malformed id leaked 16 bytes with nothing to
  reclaim them on a no-free bump. That is reachable through
  `agnosai_uuid_canonical`, which the `*_from_json` deserialisers and
  `routes/sse.cyr` call on attacker-controlled input — a request body full of
  malformed ids leaked 16 bytes each, unauthenticated. It now decodes into a
  stack buffer and allocates only on success.

  **The obvious spelling of that fix was measured and rejected.** Validating
  with one scan and then decoding with a second is the clean-looking version and
  costs **258 → 472 ns (+83%)** — `agnosai_uuid_canonical` runs on every id in a
  deserialised crew, so that is not free. One scan into a `var tmp[16]` plus a
  16-byte `memcpy` gives the same guarantee at **258 → 275 ns (+6.6%)**.
  `route_get_crew_arena` 3,821 → 3,621 ns (−5.2%).

  `agnosai_uuid_is_valid` and `agnosai_uuid_parse` share `_agnosai_uuid_scan`,
  so they cannot drift on what they accept — which matters because the routes
  decide **422-vs-404** on the validator where the oracle's `Path<Uuid>`
  extractor decided it on the parser. Eleven inputs assert the two agree.

  The scratch buffer is `var tmp[16]`, not `var tmp[AGNOSAI_UUID_BYTES]`: an
  array size must be a compile-time literal or an enum constant, and that name
  is a `var`. `tests/id.tcyr` asserts the two agree so the duplication cannot
  drift silently — **the first attempt at this claimed the constant worked**,
  because the check grepped the build output for errors without verifying a
  binary had been produced, and ran a stale one.

  **5 mutations applied, 5 caught.** `id` 41 → **65** assertions.

  Remaining, and the only read route not at zero: **`/api/v1/tools` at 288 B**,
  the per-tool `schema_fp` build. Structural until the tool vtable takes an
  allocator parameter.

- **The router's own path matching: 1.675 µs → 343 ns (−79%), and its 32 bytes
  per request → 0.** `agnosai_route_resolve` was the single most expensive part
  of a cheap request and the *only* part of an arena-threaded read route that
  still allocated. Both are gone.

  It was a flat list: sixteen `_agnosai_path_matches` calls in table order,
  every one starting from byte 0. `/api/v1/dashboard/crews` matched on attempt
  **sixteen**, having re-compared the segments `api` and `v1` twelve times on
  the way, and a request matching nothing paid all sixteen. Now `/api/v1` is
  consumed once and the segment after it selects a group of at most four
  patterns.

  What that bought, per request, on the routes B2 had already threaded:

  | route | B/req before | B/req after | µs before | µs after |
  |---|---|---|---|---|
  | `GET /api/v1/approvals` | 32 | **0** | 2.947 | **1.861** (−37%) |
  | `GET /api/v1/tools` | 320 | **288** | 3.074 | **2.344** (−24%) |
  | `GET /api/v1/agents/definitions` | 32 | **0** | 3.139 | **2.529** (−19%) |
  | `GET /api/v1/dashboard/crews` | 32 | **0** | 11.895 | **10.956** (−8%) |
  | `GET /api/v1/dashboard/agents` | 32 | **0** | 14.639 | **14.048** (−4%) |
  | `GET /api/v1/crews/{id}` | 64 | **16** | 3.957 | **3.821** (−3%) |

  **Four routes now charge the global bump literally nothing** for a request
  served through a per-request arena — handler, router and response struct all
  land in the arena and are freed by one `reset_via`. `/api/v1/approvals` gains
  most in *proportion* because resolve was 57% of it.

  Two residuals remain and neither is the router's:

  - **16 B on any `{id}` route — `agnosai_uuid_parse`.** It allocates a 16-byte
    buffer for the decoded UUID, and **all five of its callers in `src/` throw
    that buffer away**: every one is `if (agnosai_uuid_parse(x) == 0)`, a
    validity test. A request carrying an id pays it on the 404 arm as much as
    the 200 one. Not changed here — it is `src/id.cyr`, not the router.
  - **288 B on `/api/v1/tools`** — the per-tool `schema_fp` build, structural
    until the tool vtable takes an allocator. Unchanged in kind, smaller only
    because the router's 32 B came out of it.

  **The restructure is a filter, not a decision, and that is the safety
  argument.** `_agnosai_seg_is` and `_agnosai_path_prefix_end` choose which
  patterns get *tried*; what a trial *answers* is still a full pattern
  comparison. So neither selector can route a request somewhere it does not
  belong — the worst either can do is waste time. That is not a hopeful
  reading: `_t_resolve_is_equivalent` runs the new resolver against **the flat
  sixteen-attempt table it replaced**, kept verbatim in the test file, and
  requires they agree on id, captured parameter and the 405-vs-404 flag across
  56 paths × 4 methods — and making `_agnosai_seg_is` a bare prefix test
  changes **no answer it produces**, because the resolver simply degenerates
  back into the flat table.

  That property is why the two selectors get their own unit assertions: what
  they protect is the **cost**, not the routing, and the differential cannot
  see cost. Writing those caught an off-by-one in this entry's own first draft
  — `/api/v1` is seven bytes, so the separator is at index 7, and both the test
  and the source comment had said 8.

  **13 mutations applied, 13 caught**, after two rounds of strengthening: four
  on the allocator threading, six on the routing logic, three on the selectors.
  The un-threading of the *captured parameter* Str initially survived
  everything, because all four routes pinned at zero are parameterless — fixed
  by pinning `agnosai_route_resolve_a` on a `{id}` path too.

  Method **0** is in the differential corpus deliberately: `_agnosai_serve_method`
  answers 0 for any verb the table does not map, and that value reaches the
  resolver, where a known path must still answer 405.

  `server_router` 90 → **844** assertions; `server_serve` 158 → **163**.

  Also measured, not attributed: `capability_scorer_record_50caps` 288 → 317 ns.
  Nothing in `learning/` was touched and the module has ranged 270–296 ns across
  seven prior runs, so this sits just outside its band. Binary layout is the
  obvious suspect and is not evidence; recorded rather than explained away.

- **B2: every GET read route now serves from the per-request arena — the six of
  them charge the global bump 32–320 bytes per request instead of 384–4,368,
  and each is 7–21% faster.** Four bites: `llm`, `server/routes` + `server/state`,
  `tools`, and the `orchestrator` group.

  Measured on a fixture of 8 finished crews (each carrying agent metadata), 8
  pending approvals, 1 agent definition and 1 registered tool — global
  allocator against `dispatch_a` with `reset_via` between requests, which is the
  path a sandhi worker takes:

  | route | B/req global | B/req arena | µs global | µs arena |
  |---|---|---|---|---|
  | `GET /api/v1/dashboard/agents` | 4,368 | **32** | 17.110 | **14.639** |
  | `GET /api/v1/dashboard/crews` | 4,160 | **32** | 13.949 | **11.895** |
  | `GET /api/v1/crews/{id}` | 2,368 | **64** | 4.995 | **3.957** |
  | `GET /api/v1/tools` | 1,720 | **320** | 3.734 | **3.074** |
  | `GET /api/v1/approvals` | 576 | **32** | 3.158 | **2.947** |
  | `GET /api/v1/agents/definitions` | 384 | **32** | 3.911 | **3.139** |
  | `GET /api/v1/crews/{unknown}` (404) | 336 | **64** | — | — |

  **The 32 B is not the handler — it is `agnosai_route_resolve`.** The match
  struct is minted before dispatch reaches any arm, so it is in both columns and
  no amount of handler threading removes it; a `{id}` route pays 64 because the
  matcher also stores the wildcard param Str. Five of the six read routes have a
  handler half of **exactly zero**.

  *(The router entry above then took that 32 B to 0 as well. The arena column
  here is what this bite alone achieved, kept as measured rather than restated,
  so the two entries read as the sequence they were.)*

  `/api/v1/tools` is the one that does not, at 320 B, and the reason is
  structural rather than an omission: `agnosai_tool_schema` calls the tool's own
  `schema_fp`, which rebuilds its schema on whatever allocator it chooses, and
  the tool vtable has no allocator parameter. Nothing in the routes or registry
  tier can reach it.

  Threading is **not** free by construction — every `_a` call carries an extra
  argument and `alloc_via` is one indirection past `alloc` — so the latency
  columns exist to show it did not cost anything. It did not: the arena arm wins
  on all six, because the global allocator is a no-free bump whose ever-growing
  heap loses the locality a reset arena keeps.

  New `_a` forms, each with the bare name delegating through `default_alloc()`:
  `agnosai_chat_message_to_value`, `agnosai_inference_request_to_value`,
  `agnosai_inference_request_to_json`, `agnosai_chat_role_to_wire` (llm);
  `agnosai_route_dispatch`, `agnosai_route_json`, `agnosai_route_error`,
  `agnosai_route_list_definitions`, `agnosai_route_list_tools`,
  `agnosai_route_list_pending`, `agnosai_route_crew_history`,
  `agnosai_route_agent_performance`, `agnosai_route_get_crew`,
  `agnosai_app_state_definitions` (server); `agnosai_param_schema_to_value`,
  `agnosai_tool_schema_to_value`, `agnosai_tool_output_to_value`,
  `agnosai_tool_registry_list` (tools); `agnosai_orchestrator_crew_ids`,
  `agnosai_approval_gate_pending_tasks` (orchestrator).

  **`benches/server.bcyr` is new** — 13 rows, the paired global/arena timings
  above plus `route_resolve` on its own. The three earlier B2 bites shipped
  allocation numbers and no timing; this back-fills them, so all six threaded
  routes now carry both.

  **`route_resolve` is 1.675 µs, which is 57% of the cheapest threaded
  request.** Path matching is now the single most expensive part of a cheap
  route — recorded as a finding, not acted on here.

  Four traps this run, each caught by mutation and each the same shape as the
  two B2 already had on record:

  1. **`map_keys` again** (`agnosai_orchestrator_crew_ids`) — third and fourth
     sighting; slot-walked per standing rule 5.
  2. **A route's *failure* arms are separately threadable and separately
     forgettable.** With the success path measured and the 404 only
     status-checked, reverting `route_error_a` to `route_error` inside
     `get_crew_a` passed the whole suite. The 404 arm is now measured too — it
     is also the arm an unauthenticated scan hits hardest.
  3. **An empty fixture cannot tell a threaded route from an un-threaded one.**
     `/api/v1/dashboard/agents` renders an object only for a result whose
     metadata names an agent; without that metadata it emits `[]` over eight
     crews, and un-threading its per-agent object build was invisible.
  4. **A round-number threshold asserted less than it looked like it did.** 128 B
     sat comfortably above the 32 B baseline and below the 176 B of the smallest
     un-threading mutation — and still let one through, because hoisting
     `str_from_a(a, "agent")` back to `str_from` costs a single 16 B allocation
     per request and lands at 48. The bound is now `agnosai_route_resolve`'s own
     cost, measured in the same run, which makes it self-calibrating.

  **20 mutations applied, 20 caught — but three of them only after the test was
  strengthened.** Traps 2, 3 and 4 above are those three: each passed a suite
  that looked like it was measuring the thing, and each is in the record because
  the first version of this bite would have shipped with them green.
  `server_serve` 134 → 158 assertions.

  Not threaded, and not claimed to be: the write routes (`POST /api/v1/crews`,
  `/api/v1/agents/definitions`, `/api/v1/approvals`, `/api/v1/a2a`, `/mcp`),
  which parse a request body and are the other half of the problem; and the
  orchestrator group's off-request-path modules, `crew_runner` foremost.

- **The sandbox_spawn suite runs in 10.2 s against 32.6 s (−69%).** Not an
  optimisation: `_t_sp_cloexec_survives_a_grandchild` drove `sleep 30` on a
  false premise — the grandchild inherits fds 1 and 2, which cannot be
  `FD_CLOEXEC`, so the unmodified tree already waited out the full sleep and
  both of that test's assertions held whether or not the bit was set. The
  replacement closes the grandchild's stdio, leaving the errno pipe as the only
  descriptor that could hold the spawn open, and discriminates in **16 ms
  against 4 s**. One test was 30 s of a 32.6 s suite and pinned nothing.

### Security

- **`envp` entries are NUL-terminated individually rather than by luck.**
  `_agnosai_envp_build` handed `execve` each entry's raw `str_data`, which is
  not a C string — a `str_cat`-built entry's bytes are followed by whatever the
  arena holds next. **Measured: this changes nothing observable today**, because
  the bump allocator serves zeroed memory and never reuses it, so the byte after
  every entry happens to be 0. It is fixed as a latent defect, not a live one:
  a pointer passed to `execve` should not depend on an allocator's incidental
  behaviour. Both the mutation and the probe are recorded because the mutation
  *passes* — this is one of the few changes here with no test that can fail
  without it.

- **Pinned kavach 3.11.6: cx tool guests now get network isolation, closing the
  last difference from the WASI contract ADR-006 replaces.** kavach's persistent
  path — the only API with the stdin channel `cxvm` needs — applied landlock and
  seccomp and **nothing else**. It takes a `SandboxPolicy` carrying
  `network_enabled` and acted on none of it, and `config_require_namespaces`
  (3.11.5) is a `SandboxConfig` field that never reached it. So a `.cyx` guest
  could open a socket despite its policy.

  `persistent_spawn_confined_ns(command, policy, require_ns)` applies the
  policy's namespaces, opt-in and fail-closed like `process_exec` got in 3.11.5.
  `agnosai_cx_run` requests it, so `WasmSandbox`'s "only stdin and stdout" now
  holds for cx tools too.

  **The cost is stated rather than buried**: a network namespace needs a new
  *user* namespace when unprivileged, and Ubuntu 24.04 restricts those — so on
  such a host `agnosai_cx_run` reports a spawn refusal instead of running a
  guest with a network its policy denied. Mutation-verified in kavach: ignoring
  `require_ns` fails the uid-0-inside-the-namespace assertion.

- **Pinned kavach 3.11.5, closing the last two: no network isolation without a
  rootfs, and a `stderr` field that was always empty.**

  A rootfs-less sandbox got seccomp and landlock but **no namespaces**, because
  applying them unconditionally breaks payloads on hosts restricting
  unprivileged user namespaces. `config_require_namespaces` makes it opt-in and
  **fail-closed**: the payload runs inside them or exits 123/118, never
  unisolated. Off by default, so no caller changes behaviour.

  And the payload's stderr is its own stream. Both child fds went to one pipe
  and `ExecResult.stderr` was hardcoded `""` — telling a caller the payload
  wrote nothing, which is a different claim from "we did not keep it". Two
  pipes, drained round-robin, because draining one to EOF then the other
  deadlocks once the undrained pipe fills.

  Four mutations, all caught after two corrections of my own tests: a
  `contains "0"` uid check that matched **1000**, and a deadlock probe flooding
  the wrong stream — the capture caps its own buffer, so only a *small stdout
  with a huge stderr* can deadlock a drain-stdout-first loop.

- **Pinned kavach 3.11.4. Three defects agnosai found, fixed upstream, and did
  not work around — the last of them made ADR-006's premise false.**

  **Neither kavach exec path applied seccomp or landlock without a rootfs
  (3.11.3).** `process_exec` reached the confined capture only under
  `rootfs != 0`, and `persistent_spawn` took no policy at all, so
  `seccomp_enabled = 1` was stored, scored by `score_backend`, and never
  applied. Separately `landlock_rules_len` was a bare counter with no list
  behind it and `confine_child` passed `security_apply_landlock(0, 0)` — so
  landlock was applied by nothing.

  This is exactly what [ADR-006](docs/adr/006-cx-tool-sandbox.md) rests on:
  `cxvm` dispatches guest syscalls straight to the host kernel, so kavach's
  seccomp + landlock *are* the whole boundary. Measured with the ADR's own
  acceptance test — a `.cyx` calling `open("/etc/passwd")` and reporting the raw
  syscall return:

  | | unwrapped `cxvm` | via kavach |
  |---|---|---|
  | 3.11.2 | fd 3 | **fd 3** |
  | 3.11.3+ | fd 3 | **EACCES** |

  Fixed by routing confinement on the policy rather than the rootfs (fail-closed
  when a requested filter cannot be built), adding `policy_landlock_add` /
  `policy_landlock_deny_all`, and adding `persistent_spawn_confined`, which
  installs landlock and seccomp between the pipes and `execve` — landlock does
  not restrict already-open descriptors, which is what lets a confined guest
  still read its stdin. `persistent_spawn` keeps its signature and its
  unconfined behaviour, now documented rather than implicit.

  **`SandboxConfig.timeout_ms` was ignored by every backend except WASM
  (3.11.4).** A 1000 ms deadline let `/bin/sleep 8` run **8001 ms** and report
  `timed_out = 0` — unenforced, and the flag lying about it. `confine_capture`
  now takes a deadline, drains non-blocking with a 5 ms idle sleep, and SIGKILLs
  then reaps on expiry.

  **The process backend never reported the payload's exit code (3.11.4).**
  `/bin/false` (real 1) and `/bin/ls` on a missing path (real 2) both came back
  **0**, so a failing payload was indistinguishable from a successful one. Both
  capture paths now share one implementation, so they cannot drift apart again.

  Six mutations across the two releases, all caught by kavach's suite.

  **One regression of my own, caught by CI rather than locally**: routing on the
  policy pulled namespace creation into a path that never had it, and
  unprivileged user namespaces are restricted on stock runners — so payloads
  that previously ran stopped running. Namespaces are now applied on the rootfs
  path only, via a `want_ns` parameter that leaves every pre-3.11.3 caller
  untouched, including `sandbox_spawn`'s fail-closed namespace contract.

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

- **M7 bite 14 — `sandbox/cx`'s execution half: `.cyx` inside a kavach
  sandbox.** `agnosai_cx_run` and `agnosai_cx_compile_and_run`, 62 assertions.
  **ADR-006's acceptance test is now a suite assertion**, not a scratch probe: a
  `.cyx` calling `open("/etc/passwd")` is refused.

  **There is no unconfined branch.** ADR-006's hard requirement is that no code
  path executes a `.cyx` outside a kavach sandbox, because `cxvm` dispatches
  guest syscalls straight to the host kernel with no allowlist. The runner has
  no flag and no fallback — a sandbox that cannot be created is a refusal, not a
  reason to run the payload anyway. Mutation-verified: swapping
  `persistent_spawn_confined` for `persistent_spawn` fails the gate.

  **Landlock allows the interpreter's directory and nothing else — not
  `deny_all`.** A total-deny ruleset stops the child opening `cxvm` itself, so
  the guest exits 127 having never run: a refused *exec* that reads exactly like
  a refused *open*. The gate test asserts `code != 127` for that reason, and the
  `deny_all` mutation fails on it.

  **The deadline is enforced here, not delegated.** kavach's persistent API
  takes no timeout and `SandboxConfig.timeout_ms` is ignored by every backend
  except WASM (filed upstream), so a runner that delegated would have no bound
  at all — and a wall-clock bound is the *only* CPU bound cx has, fuel having
  gone with wasmtime. Removing it hangs the suite.

  Four mutations, all caught: unconfined spawn, `deny_all` instead of a rule,
  no deadline (hangs), and skipping bytecode re-validation.

  **Not claimed: network isolation.** kavach applies namespaces only to
  rootfs-bearing sandboxes, and this one has no rootfs, so a guest can still
  reach the network. Written into the module rather than left implied.

- **M7 bite 13 — `sandbox/cx`: compiling tool source to cx bytecode.** 33
  assertions. The compile half of `wasm.rs`'s successor per ADR-006 — source in,
  `.cyx` out, through `cycc_cx`.

  **`cycc_cx` speaks stdin and stdout, not paths.** `cycc_cx <path>` hangs
  forever waiting on a stdin that never closes; the working form is
  `cycc_cx < source > out.cyx`, which is what the spawn primitive's
  `capture_input` already does — with the sanitized environment and a deadline,
  since a compiler fed untrusted source is itself a subprocess on
  attacker-influenced input.

  **Float literals are rejected before the compiler runs.** ADR-006 records that
  cx miscompiles them until arc B and asks the loader to reject rather than
  trust. Measured: `var x = 1.5;` compiles with **exit 0 and 160 bytes of
  output** — nothing in the result to test afterwards. Strings and comments are
  skipped, so `# 1.5x faster` and `"version 1.5"` are not rejections.

  **The magic is `CYX\x01`, not `CYX\0`.** ADR-006 says `"CYX\0"`; a real
  `cycc_cx` emits `43 59 58 01` — `CYX` plus a format version. Checking the
  ADR's spelling would reject every real `.cyx`.

  Four mutations, all caught after one correction: dropping the float check,
  ignoring the compiler's exit code, skipping the version byte, and letting
  prose trip the float checker.

- **M7 bite 12 — `kavach_bridge`'s exec half.** `agnosai_kavach_execute` —
  `kavach_bridge::execute` (`kavach_bridge.rs:94-148`) — 89 assertions in the
  suite. Create, transition to Running, exec, then destroy on **every** path
  before propagating an exec failure, which is the oracle's ordering and not an
  accident of it.

  **kavach's backends are registered lazily on first use.** Without
  `kavach_init` every `sandbox_create` fails with "backend not available", and
  nothing else in agnosai calls it. Doing it in the bridge keeps the module
  self-contained; `alloc_init` has been idempotent since cyrius v6.1.23, which
  is what makes lazy initialization safe rather than a 256 MB leak per call.

  Five mutations, three caught: skipping the lazy init, re-scanning the output
  instead of leaving `scan_verdict` as None, and computing the strength score
  from a default policy rather than the caller's. The two that survive are
  recorded in the code and the test — see below.

- **M7 bite 11 — `sandbox/manager`: backend selection and dispatch.** All eight
  oracle tests ported (`manager.rs:145-217`), 69 assertions. Every module in the
  oracle's M7 sequence — `policy`, `oci`, `kavach_bridge`, spawn, `process`,
  `python`, `manager` — now has a Cyrius counterpart.

  **Dispatch is on the EFFECTIVE isolation level, not the requested minimum.**
  `resolve_backend` forwards to the policy and is easy to test; the dispatch path
  computes the same thing separately, and a version reading `min_isolation` there
  would send a policy that asks for `None` but needs the filesystem straight to
  the passthrough — silently running an unsandboxed tool with no sandbox at all.
  The first test round missed this, because it only exercised `resolve_backend`.

  **`IsolationLevel::None` echoes the input and never touches `argv`**
  (`manager.rs:87-93`). A native tool is already in-process, so there is nothing
  to spawn; the passthrough is what "no sandbox needed" looks like from here. The
  test asserts a marker that would appear if `argv` had run, so the two outcomes
  are distinguishable rather than both looking like success.

  **`max_duration_secs == 0` means "unset", not "unlimited".** The policy's
  deadline wins when set and falls back to the manager's 30 s otherwise
  (`manager.rs:80-84`); the other reading would let a tool run forever, so both
  arms are asserted against a child that would sleep for a minute.

  Seven mutations, all caught: ignoring the policy deadline, treating an unset
  deadline as unlimited, running `argv` on the `None` path, dispatching WASM to a
  backend instead of erroring, skipping OCI image validation, ignoring
  `needs_network`, and dispatching on `min_isolation`.

- **M7 bite 10 — `sandbox/python`: the interpreter bridge.** All five oracle
  tests ported (`python.rs:177-312`), 61 assertions. The third and last consumer
  of the spawn primitive.

  **The tool source travels on stdin, never in the script**, and the test proves
  it against a real interpreter. The wrapper is a constant; the source arrives as
  `__tool_source__` in the JSON payload, so a source carrying `"""` and its own
  `print` cannot close the quoting and append a statement. Mutation-verified by
  rebuilding `execute_tool` in the vulnerable shape — the source interpolated
  into a triple-quoted literal — at which point the injected statement runs and
  the test fails.

  The oracle's module doc (`python.rs:4`) claims "seccomp + Landlock + cgroups
  + network namespace"; the "(future)" on that line is doing all the work, and
  none of it exists. kavach supplies the real confinement here, which is a
  milestone gate rather than something this module quietly does.

  Four mutations, three caught after correction: interpolating the source into
  the script, keeping partial output on timeout, and removing PATH resolution.
  The fourth — swapping the sanitized `envp` for the raw one — cannot be caught,
  because a test process has no `setenv` with which to plant an `LD_PRELOAD`;
  that limitation is written into the suite rather than left implied.

- **M7 bite 9 — `sandbox/oci`'s exec half.** `agnosai_oci_execute` —
  `OciSandbox::execute` (`oci.rs:106-187`) — 100 assertions in the suite. Bite 2
  had already built the argv as a testable value, so this runs that argv through
  the spawn primitive and maps the result.

  **The runtime inherits the server's environment unfiltered, matching the
  oracle.** `execute` has no `env_remove` loop: `SANITIZED_ENV_VARS` belongs to
  the *subprocess* backends (`mod.rs:6`), and a container's environment comes
  from `--env` arguments rather than inheritance. Filtering here would sanitize
  the trusted `docker`/`podman` binary the operator installed and change nothing
  about what the container sees.

  **`timeout_secs` is the oracle's unit and the primitive takes milliseconds.**
  A missing ×1000 would make every container time out a thousand times early;
  the test asserts the elapsed deadline is a second, not a millisecond.

  Four mutations, all caught: dropping the ×1000, keeping partial output on
  timeout, treating a spawn failure as a normal result, and assembling a second
  argv inside `execute` instead of using `build_argv` — which is how the
  `--read-only`, `--tmpfs` and `--network=none` posture would silently vanish.

  The suite writes a small stand-in runtime rather than assuming a container
  runtime exists. The obvious substitutes do not work: `build_argv` puts
  `run --rm -i ...` ahead of everything, so `cat` tries to open a file named
  `run` and `sleep` rejects it as a bad operand.

- **M7 bite 8 — `sandbox/process`: the subprocess sandbox.** All nine oracle
  tests ported (`process.rs:250-366`), 108 assertions in the suite. This is the
  backend `IsolationLevel::Process` resolves to, and the first consumer of the
  finished spawn primitive.

  **`work_dir` and a failure reason were added to `spawn` to serve it.** The
  `chdir` happens in the child between `fork` and `execve` — a parent-side one
  would relocate the whole server — and a directory that cannot be entered fails
  the spawn instead of running the child somewhere else. The errno pipe already
  carried eight bytes the parent read and discarded; naming them distinguishes a
  bad `work_dir` from a missing executable, so an operator's typo no longer
  reports as a generic exec failure.

  **The timeout arm discards partial output, deliberately.** The oracle returns
  `String::new()` for both streams (`process.rs:154-160`) because cancelling
  `wait_with_output` throws the buffers away. The spawn primitive *has* them, so
  the blanking is explicit at this layer rather than implicit below it.

  Six mutations, five caught: keeping the partial output on timeout, taking the
  executable from config in argv mode, dropping the empty-argv guard (aborts on
  `vec: index out of bounds`), applying `config.env` before the base instead of
  over it, and ignoring `clean_env`. The sixth is recorded under Fixed.

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

- **`agnosai_process_config_set_env` read a `Str` as a C string.** The
  `NAME=value` splitter it reused walks a raw `char*` from `environ`; handed a
  `Str`, it read the 8-byte length header as characters, so setting the same
  variable twice appended a duplicate instead of replacing it and the child got
  two entries to choose between. A `Str`-shaped sibling now sits beside the
  cstr one.

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
