# AgnosAI — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures
> (durable); this file is **state** (volatile).
> Last refreshed: 2026-08-10.

## Version

**1.1.0** (`VERSION`) — the last shipped Rust release, now preserved at
`rust-old/`. The Cyrius line targets **v2.0.0**; VERSION bumps once parity
lands, not before, so the number always names something that actually shipped.

## Toolchain

- **Cyrius pin**: `6.5.18` (`cyrius.cyml`) — bumped 2026-08-10, three-step,
  `lib/` diffed after the sync AND again after a build: 107 files, zero content
  differences. Folds **sigil 3.12.7**, **sakshi 2.4.10** and **sankoch 2.7.7**.

  **Sibling pins as of the 2026-08-10 bump**: kavach **3.11.9**, majra **2.6.1**,
  sigil 3.12.7, bote 3.3.0, ai-hwaccel 2.3.16, tyche 1.0.0, and the defensive
  `[deps.sakshi]` at 2.4.10.

  ⚠ **The defensive `[deps.sakshi]` is still load-bearing, and its comment has
  now been wrong twice about WHY.** It first blamed "sigil and bote", then
  "majra 2.6.0". sigil dropped its `[deps.sakshi]` at 3.12.7 and majra at 2.6.1;
  the sibling that still declares one is **bote 3.3.0, at sakshi 2.4.7**. Do not
  trust the manifest comment — re-derive with
  `grep -l '^\[deps\.sakshi\]' ../*/cyrius.cyml`.

  ⚠ **Every build emits 35 `duplicate fn … (last definition wins)` warnings.**
  Audited 2026-08-10: **none is currently miscompiling**, two are latent hazards,
  and the audit's own first two conclusions were wrong. The corrections are in
  CHANGELOG and matter more than the finding:

  - **`err_io` (kavach ∩ sigil) does NOT diverge.** sigil has two unrelated error
    enums and comparing them with prefixes stripped merges them. The syscall-error
    tables are identical on all eight shared kinds. Probed: `err_io(5,…)` → kind
    8, errno 5.
  - **`_sub_new` (majra ∩ libro) works here.** Probed: `sub + 8 == 0`, so majra's
    body runs, and `pubsub_publish` delivers. Correct **by accident of include
    order** — both directions corrupt if it flips. agnosai is doubly safe:
    `src/orchestrator/pubsub.cyr` calls no majra `pubsub_*`.
  - Genuinely latent: **`attestation_result_new`** (kavach ∩ sigil), two unrelated
    functions sharing a name, inert only because neither library calls it.

  ⚠ **The warning's `file:line` is NOT usable to decide which copy wins** —
  `lib/kavach.cyr:11512` names a file of 11,321 lines, so the offsets are into the
  preprocessed stream. Both wrong conclusions above came from trusting it. Filed
  to cyrius. **Run the symbol; do not read the warning.**

  ⚠ **A caller parsed BEFORE a redefinition still binds to the later body.**
  Measured (`early_caller()` returns 22, not 11). This is why a duplicate is never
  "harmless because our copy comes first" — but it is also not enough on its own to
  say which copy that is.

  Previously 6.5.14 (sigil 3.12.6).

  **6.5.14 is not optional here.** It fixes the tail-call frame-release bug in
  standing rule 12, and sigil 3.12.6 — which fixes an **RSA-PSS authentication
  bypass**, the same class agnosai reported against PKCS#1 v1.5 — declares
  `cyrius >= 6.5.14` as its floor. agnosai reaches PSS through TLS 1.3
  CertificateVerify (`lib/tls_native_hs13.cyr:257,260`) on every outbound HTTPS
  call from `tools/agnos.cyr`, `tools/remote_registry.cyr` and
  `guarded_fetch.cyr`, so holding at an older sigil would ship a TLS peer-auth
  bypass.

  ⚠ **`sigil.cyr` is BOTH folded into the snapshot AND a git dep with
  `path = "../sigil"`, and when the sibling is ahead of the fold those two
  demands conflict.** The three-step leaves `lib/` matching the snapshot, then
  **every `cyrius build` re-layers the sibling's dist** — build performs an
  implicit resolve and the local path beats the tag. Seen live on 2026-08-08
  with the fold at 3.12.5 and the sibling at 3.12.6. A `check-clean` allowance
  was written for it and then **deleted unused**, because the next toolchain
  snapshot carried 3.12.6 and the conflict evaporated. If it recurs, the choice
  is a narrow newer-only allowance with a stated exit condition, or dropping
  `path` so local resolution matches CI — not silently living with a red gate.

  ⚠ **Do not trust a single `lib sync --full` when the toolchain was installed
  moments earlier.** The 6.5.13 bump's first sync reported `copied 107 .cyr
  files` and left `lib/syscalls_x86_64_agnos.cyr` at the previous pin —
  `check-clean` caught it, a second sync fixed it. **Always diff `lib/` after
  syncing, and again after any `deps` run.**

  Previously 6.5.13 (sigil 3.12.5), 6.5.12 (3.12.4), 6.5.11.

- **`lib/` matches the pin exactly**: 0 of 106 stdlib files differ from
  `~/.cyrius/versions/6.5.10/lib`; build and test emit no drift or shadow warning.
  The six files outside the snapshot (ai-hwaccel, bote-core, kavach, libro, majra,
  tyche) are declared git deps, not staleness.

  **`lib/unicode/` is vendored BY HAND and nothing upstream maintains it.**
  `cyrius lib sync` copies only the top level, so declaring `"unicode"` in
  `[deps].stdlib` — which agnosai now does — does **not** bring the seven files
  under `lib/unicode/`. They were copied from the snapshot manually.

  It stopped being inert on 2026-08-03: `src/sandbox/oci.cyr` calls
  `unicode_category` to match the oracle's Unicode `is_alphanumeric` exactly, so
  those files are load-bearing. `scripts/check-clean.sh`'s snapshot check is
  therefore **recursive**, and is the only thing that will notice if they drift
  from the pin — mutation-verified against a one-byte edit.

  **Verify a bump by diffing the trees, not by a green `lib sync --full`.**
  `cyrius lib sync` skips on file **size**, not content, so a size-neutral change
  — in practice a version stamp — is never copied while the command reports
  success. The 6.5.6 bump left `lib/vani.cyr` a patch behind that way. **Impact
  here was nil** (vani is neither declared nor called), and that is the usual
  shape: real code edits change length and do sync. Filed as a papercut.
  `scripts/check-clean.sh` diffs `lib/` against the pinned snapshot, which
  `deps --verify` cannot do — the lock is written *from* disk, so a stale file
  just gets its stale hash recorded.

### Pins must name what is actually built — corrected 2026-08-03

`cyrius.cyml` pinned **bote 3.2.1** and **kavach 3.9.3** while `lib/` held **bote
3.3.0** and **kavach 3.11.0**. Both vendored bundles are byte-identical to their
upstream tag dists (verified by sha256 against `git show <tag>:dist/...`). The pins
are now **bote 3.3.0** / **kavach 3.11.0**, which is what was already being compiled
and tested.

**The mechanism, because it will recur.** Every `[deps.NAME]` carries
`path = "../NAME"` alongside `git` + `tag`, and **the local path wins**. A developer
whose sibling checkout has moved ahead silently builds a version the manifest does
not name; CI, which has no sibling checkouts, resolves the *tag* and builds
something else. Here that was kavach **3.11.0 locally against 3.9.3 in CI** — the
`~/.cyrius/deps/kavach/3.11.0/` clone predates the session that found it.

This is the inverse of the sigil rule recorded below, and both are live: a stale tag
can *overwrite* a newer folded copy (sigil's case) **or** be quietly overridden by a
newer local path (kavach's case). The lockfile does not save you from the second —
`cyrius.lock` recorded 3.9.3's sha256 against a 3.11.0 file on disk and nothing
surfaced it, because every other gate reads `src/` and the build compiles whatever
bytes `lib/` holds.

**Two gates now close the class**, both mutation-verified:

- `scripts/check-clean.sh` runs **`cyrius deps --verify`** (lock vs. working `lib/`).
  Restoring the stale hash makes it print `FAIL: lib/kavach.cyr (hash mismatch)` and
  exit **1**. Note the exit code must be read directly — piping through `tail` reports
  `tail`'s status and reads as a pass.
- CI gains a **Lockfile is honest** step: `git diff --exit-code -- cyrius.lock` after
  `cyrius deps`, which catches a committed lock that disagrees with a clean tag-only
  resolution — i.e. exactly the local/CI divergence above.

**What the two bumps actually change: nothing on any path agnosai executes.**
kavach 3.10.0/3.11.0 are `--agnos` target build fixes — nine additive `kv_*` shims
(`kv_unlink`, `kv_rmdir`, `kv_waitpid`, `kv_getgid`, `kv_lstat`, `kv_fork`, `kv_dup2`,
`kv_execve`, `kv_setsid`); agnosai calls only `score_agent`, `score_agent_with_tools`,
`sandbox_display`, `sandbox_strength`, and the diff touches none of them. bote 3.3.0
adds `dispatcher_set_server_info` and is additive by construction — an unconfigured
dispatcher emits the pre-3.3.0 wire byte for byte. The duplicate-fn warning count is
unchanged at **35**, and all 57 suites stayed green across the bump.

### What 6.5.6 lands on agnosai

- **sandhi 1.9.9 — the serve-loop stop facility agnosai filed.**
  `sandhi_server_options_stop_flag(opts, ptr)` on all five loops, returning 0 for
  a requested stop against 1 for a failure. This is what made graceful shutdown
  possible; consumed immediately in `src/server/serve.cyr` +`src/main.cyr`, and
  recorded as [ADR 013](../adr/013-graceful-shutdown-via-signalfd-and-stop-flag.md).
- **`sys_exit_group`** (`lib/syscalls_linux_common.cyr:155`) — also agnosai's
  filing. `_agnosai_exit_process` now composes it instead of a hand-rolled
  `syscall(SYS_EXIT_GROUP, …)`; the `#ifdef CYRIUS_TARGET_LINUX` guard stays,
  because that file is included only by the two Linux target files.
- **`async_await_readable_ms`** — agnosai's filing, found while building sandhi
  1.9.9. Not used here; it removes the reason sandhi's cooperative loop polls.

### What 6.5.5 landed

- **bayan 1.4.0 completes the `_a` JSON surface** — `obj_set_a`, `build_a`,
  `build_pretty_a`, `parse_a`, `parse_buf_a`, `parse_ctx_a`, joining the eight value
  constructors that already had `_a` forms. This is the one that mattered: before it,
  a per-request arena could hold part of a response tree and the pair cells, the parse
  and the serialized text still landed on the no-free global bump. **Blocker #3's
  handler half is now closed** and needs no further upstream work. First consumer:
  `src/core/task.cyr`, measured **1792 → 0 bytes/response**. See roadmap B3.

### Carried forward from 6.5.4

- **`vec_sort_by` / `vec_select_nth`** closed agnosai's own sort filing.
  ✅ **Consumed** — `src/order.cyr` is now a 98-line wrapper over them (was 184).
- **sandhi 1.9.7 → 1.9.8** changed a **return contract** the transport tier depends on:
  all five serve loops previously spun a core forever on a persistent accept error and
  never returned once listening; they now return 1 on a structurally dead listener or
  after 200 consecutive resource failures. `src/server/serve.cyr` is the caller.
- **sigil 3.12.1 → 3.12.2** fixed a 144-byte-per-call `sha256_init` leak. **It never
  affected us** — `_agnosai_auth_secret_eq` uses the banked `sha256()` one-shot, which
  is allocator-free — confirmed by measuring the shared-secret path at **32
  bytes/request** before and after.

  **The sigil pin has to move with the fold.** `cyrius deps` copies each git dep's
  vendored bundle into `lib/` with last-write-wins, so leaving `[deps.sigil] tag =
  "3.12.1"` would overwrite the fold's 3.12.2 back down to 3.12.1.
- Rust (for `rust-old/` only): `channel = "stable"`, currently rustc 1.96.0

## Source

`src/` mirrors `rust-old/src/` — see CLAUDE.md's *Layout* rule. Regenerated from
the tree 2026-08-08 with `find src -maxdepth 1 -name '*.cyr'` per group:

| Group | Files | Lines | Oracle |
|---|---|---|---|
| `orchestrator/` | 16 | 5,632 | ✅ complete (M5) |
| `server/` | 11 | 4,291 | ✅ complete (M6) |
| `fleet/` | 12 | 3,676 | ✅ complete (M8) |
| `telemetry/` | 3 | 1,524 | ✅ complete (M9) — `mod` + `genai` + OTLP, with the span call sites wired ([ADR 017](../adr/017-genai-span-call-sites.md)) |
| `sandbox/` | 9 | 3,345 | ✅ complete (M7) |
| `core/` | 8 | 3,305 | ✅ complete (M2) |
| `server/routes/` | 10 | 2,686 | ✅ complete (M6) |
| `tools/builtin/` | 7 | 2,138 | ✅ complete (M4) |
| `llm/` | 4 | 1,229 | ✅ complete (M3) |
| `tools/` | 5 | 1,144 | ✅ complete (M4) |
| `learning/` | 6 | 1,018 | ✅ complete (M2) |
| root (port-local) | 6 | 1,054 | no oracle — `main`, `units`, `order`, `id`, `guarded_fetch`, `chan_lossy` |
| **total** | **97** | **31,163** | against a **41,163**-line oracle |

Regenerate with:

```sh
find src -name '*.cyr' | xargs wc -l | tail -1
```

⚠ The oracle denominator is **41,163**, not the 27,683 this file carried until
2026-08-07 — see the banner below for what the smaller number leaves out.

**Oracle groups still owed** (`sandbox/` completed at M7, so this table is
narrower than it was; the scope qualifiers on the rest are **removed** — see the
corrected section below):

| Group | Rust lines | Milestone | Scope |
|---|---|---|---|
| ~~`definitions/`~~ | ✅ 0 left | M10 | **COMPLETE 2026-08-09** — all six modules, 1,460 lines. Both former blockers were already in `lib/`: YAML came from bayan's parser, and ZIP needed one line in `cyrius.cyml`'s `[deps].stdlib` rather than the upstream sankoch ask the roadmap predicted. |
| ~~`telemetry/`~~ | ✅ 0 left | M9 | **Source-complete 2026-08-09** — `mod.rs` (116, with the JSON logging init) and `genai.rs` (206). What remains of M9 is not an oracle file: the **OTLP exporter body**, copied from hoosh's `otlp.cyr`, behind the branch `agnosai_telemetry_init_tracing` already takes. ⚠ The planned **sakshi filing for JSON output was never needed** — `sakshi_set_emit_hook` already exists; see roadmap M9. |

`fleet/` left this table on **2026-08-08**: all twelve of the oracle's modules
are ported.

**M11 bite log — ✅ COMPLETE 2026-08-10, 6 of 6.** Both `hwaccel` halves,
`tools/python_tool`, `sandbox/wasm`, `tools/wasm_tool`, `tools/wasm_loader` —
**48 mutation probes, 48 kills**. ⚠ `wasmtime` is a host requirement and is not
installed here, so the execute path's end-to-end arm is guarded; everything else
— the manifest parse, the header validator, the exit-code classifier, the SDK
stdin wrapper and both result ladders — is reachable without it, because each
was split out of its caller for exactly that reason.

**M11 bite log — 🟡 4 of 6, 2026-08-10.** `sandbox/wasm` (43 assertions, 10/10
mutants) landed on kavach 3.11.8's WASM backend. ⚠ Three upstream round trips
closed to get there: kavach 3.11.8 (backend reachable, stdin, real exit codes),
sankoch 2.7.7 (`zip_bound`, `zip_last_error`, arena-backed readers) and cyrius
6.5.17 (the `distlib` self-check). All three are consumed, not merely filed.

**M11 bite log — 🟡 3 of 6, 2026-08-09.** `llm/router` hwaccel (2 fns, 15
assertions), `core/resource` hwaccel (6 items + a `sched_getaffinity` core
count, ~50 assertions), `tools/python_tool` (50 assertions). 25 mutation probes,
25 kills. Remaining: `sandbox/wasm.rs` (521 — **absent from the roadmap's M11
row until now**), `tools/wasm_loader.rs` (169), `tools/wasm_tool.rs` (265).
⚠ `wasmtime` is **not installed** on this box and kavach's WASM backend cannot
carry stdin or a guest exit code; see the roadmap's M11 row.

**M10 `definitions` bite log — ✅ COMPLETE 2026-08-09.**

| module | oracle lines | assertions | oracle tests |
|---|---|---|---|
| `versioning` | 252 | 45 | 8 |
| `assembler` | 236 | 40 | 7 |
| `k8s_crd` | 238 | 84 | 4 |
| `loader` | 415 | 127 | 15 of 17 |
| `loader` presets | (`src/presets/*.json`) | 137 | the other 2 |
| `packaging` | 303 | 127 | 7 |
| `mod` | 16 | — | — |
| **total** | **1,460 (100%)** | **560** | **43** |

Two things landed alongside the ports rather than inside them. **`src/strcase.cyr`**
is a port-local root module extracted at the third caller
(`server/prompt_guard`, `fleet/cost_planning`, `definitions/assembler`) — no
oracle counterpart, since Rust gets it from `eq_ignore_ascii_case`; 41
assertions. **`src/definitions/presets_data.cyr` is GENERATED** by
`./scripts/gen-presets.sh` from `src/presets/*.json` and committed;
`scripts/check-clean.sh` runs the generator's `--check` so the two cannot drift.
It stands in for the oracle's eighteen `include_str!`s, which Cyrius has no
equivalent for.

`packaging` is the only module in the group that swaps a dependency rather than
porting logic: the oracle's `zip` crate becomes `lib/sankoch.cyr`, and the two
disagree about member names, decodable methods, encrypted archives and whether
the 1 MiB guard is enforced. Seven differences,
[ADR 018](../adr/018-sankoch-path-check-on-import.md), every one of them in the
stricter direction and every one pinned by a test.

⚠ **`GET /api/v1/presets` answers eighteen presets now, not `[]`** — and the
handler moved from `server/routes/tools.cyr` to `server/routes/definitions.cyr`,
which is where the oracle keeps it. The old `[]` was read off the oracle's
*default cargo build*, where `default = []` gates the `definitions` feature off;
this port has no features and ships the whole crate, so the populated arm is the
true one. `tests/server_router.tcyr`'s assertion was inverted with it.

**M8 `fleet` bite log — ✅ COMPLETE 2026-08-08.** Each module carries its own
`.tcyr`; the assertion counts are the suites', not the oracle's:

| module | oracle lines | assertions | oracle tests |
|---|---|---|---|
| `cost_planning` | 275 | 48 | 10 |
| `registry` | 573 | 67 | 20 |
| `placement` | 443 | 58 | 12 |
| `gpu` | 445 | 65 | 15 |
| `state` | 558 | 95 | 17 |
| `environment` | 261 | 62 | 8 |
| `relay` | 350 | 58 | 9 |
| `discovery` | 174 | 29 | 7 |
| `coordinator` | 530 | 81 | 17 |
| `federation` | 587 | 119 | 19 |
| `topology` | 219 | 39 | 6 |
| `mod` | 28 | — | — |
| **total** | **4,443 (100%)** | **721** | **140** |

Two of the group's bites were not just ports. `relay` is a genuine wrapper over
majra only because four upstream defects were fixed first (**majra 2.6.0**);
`coordinator` and `federation` each reproduce oracle arithmetic that reads like
a bug — `max_retries` allowing one fewer retry than it names, and
`declare_coordinator` adopting any term that is not stale.

**`src/main.cyr` is no longer a stub** — as of 2026-08-03 it wires the shared
state, installs a `signalfd` SIGINT/SIGTERM handler, and calls `agnosai_serve`
(roadmap A2, closed). **It drains on shutdown**
([ADR 013](../adr/013-graceful-shutdown-via-signalfd-and-stop-flag.md)); ADR 012,
which recorded that as impossible, is superseded — the blocker was fixed upstream
in sandhi 1.9.9 on agnosai's own filing, hours after it was written.

## Where the port is

**M0–M11 complete. `src/` is FULLY PORTED as of 2026-08-10** —
`llm/inference_queue` was the last module without a Cyrius counterpart, and
`src/llm/mod.cyr`'s claim that it "defers with that feature" was wrong on the
standing rule that a cargo feature gate is not a scope boundary.

**M12 is in progress** and is no longer about `src/`. What is left is the test
and bench surface plus the non-`src` artifacts; `roadmap.md`'s M12 section
carries the measured table and the per-bite status. The headline numbers:
**863 oracle test fns across 84 modules**, **117 oracle criterion bench ids
across 19 files**, against 96 Cyrius suites and 7 `.bcyr`.

⚠ **The bench gate was broken and nobody noticed.** Three of the six `.bcyr`
files did not compile — stale include lists against `src/` — so `cyrius bench`
reported `5 passed, 3 failed` and **50 of the tree's 79 benchmarks, including
the entire 35-shape orchestration set, had stopped running.** Fixed 2026-08-10.

The mechanism matters more than the fix. `cyrius bench` **exits 1** on a compile
error and `bench-history.sh` runs it under `set -euo pipefail`, so it would have
aborted rather than recorded partial rows — the gate was never silent, it was
**never invoked**. Last recorded run: 2026-08-07, 89 rows. The structural hole
was that `check-clean.sh` did not sweep `benches/` and CI never compiled it, so
the only thing between a rotted benchmark and nobody noticing was someone
remembering to run the script. Both closed: `.bcyr` joins the fmt and lint loops,
and CI has a `Benchmarks` step for the compile (not the numbers — CI timings are
too noisy to gate on, and `bench-history.csv` is recorded from a quiet machine).

⚠ **The oracle-test screen came back nearly clean and that is not proof.** It
matches oracle fn-name tokens against suite prose. It found a whole missing
suite (`routes/sse`), so it earned its keep, but a per-module read of oracle
test bodies against suite assertions has **not** been done.

**M0–M6 detail.** The server tier is done: bite 16 (bind) and bite 15c (SSE)
both landed 2026-08-03, so `./build/agnosai` binds, reads the oracle's
environment, answers the full route table, **streams crew events**, and drains
on SIGINT/SIGTERM.

`/api/v1/crews/{id}/stream` now streams. The 501 in
`agnosai_route_dispatch` is unreachable over the transport —
`agnosai_serve_handler` intercepts the route first, because a dispatcher that
returns "a status and a finished body" cannot express a stream. It remains the
honest answer for a direct dispatch, which is what `tests/server_router.tcyr`
still asserts.

**One divergence worth knowing before deploying**: an SSE stream holds one of
the 100 pool workers for its whole life, where the oracle serves effectively
unbounded concurrent streams —
[ADR 014](../adr/014-sse-stream-holds-a-pooled-worker.md) has the tower analysis
showing why the "the oracle starves at 100 too" intuition is wrong.

Verified live, not inferred: `/health` → 200 `{"status":"ok"}`, `/metrics`
renders the registry, `/api/v1/tools` lists the four registered builtins, and
the stream route returns 501. `PORT` / `AGNOSAI_PORT` / `HOOSH_URL` and the four
auth variables were each exercised against a running binary — see the CHANGELOG
for the eight port cases and nine auth cases and what each proves.

Standing decisions that shaped the port, each with its record:

| Decision | Where |
|---|---|
| Money is integer micro-USD, converted to f64 only at the wire | port plan, open question 1 |
| Tool sandbox rides **cx + kavach**, not WASM | [ADR 006](../adr/006-cx-tool-sandbox.md) |
| LLM reached over an **HTTP seam**, not linked | [ADR 003](../adr/003-llm-native-http.md) |
| SSRF guard re-runs on every redirect hop (oracle checks only the first) | [ADR 007](../adr/007-audit-redirect-revalidation.md) |
| Auth compare is SHA-256-digest-based, not the oracle's length-leaking loop | [ADR 009](../adr/009-auth-constant-time-secret-compare.md) |
| JWT requires configured `iss`/`aud` | [ADR 010](../adr/010-jwt-require-configured-iss-aud.md) |
| `/metrics` serves agnosai's registry, not hoosh's | [ADR 011](../adr/011-metrics-endpoint-serves-agnosai-metrics.md) |
| `agnosai_chan_push_lossy` gives tokio-broadcast evict-oldest semantics | `src/chan_lossy.cyr` |

## Tests

**96 suites, all passing** (2026-08-10). The five most recent:
`tests/llm_inference_queue.tcyr` (69 assertions, 18 mutation probes / 18 kills),
`tests/server_routes_sse.tcyr` (31 — the routes tier's one module that had no
suite), `tests/integration_crew_with_tools.tcyr` (29 — the oracle's own
`rust-old/tests/` integration file, invisible to a per-module screen),
`tests/orch_orchestrator.tcyr` (55, up from 51: it only ever ran ONE crew, so a
single-slot registry passed it), and `tests/tools_wasm.tcyr` from M11.

⚠ **`cyrius tests` runs in CI; until 2026-08-10 `cyrius bench` did not.** Both
report `N passed, M failed` and both exit non-zero, so neither is silent — but
only the test gate was wired into a pipeline. Three `.bcyr` files broke in the
three days after the 2026-08-07 bench run and nothing invoked the gate again.
CI now has a `Benchmarks` step. Read the `N passed, M failed` line on both.

**65 suites, all passing** was true at the M7 audit, whose 43 findings are **all fixed** as of
2026-08-05 — `docs/development/m7-audit-2026-08-04.md` marks each one in place
and `roadmap.md`'s M7 section has the shape of the remediation. The sandbox
suites grew **627 → 727 assertions** working it:

| suite | before | after |
|---|---|---|
| `sandbox_policy` | 90 | **102** |
| `sandbox_oci` | 100 | **118** |
| `sandbox_kavach_bridge` | 93 | **136** |
| `sandbox_spawn` | 144 | **169** |
| `sandbox_process` | 108 | **130** |
| `sandbox_python` | 61 | **76** |
| `sandbox_manager` | 69 | **90** |
| `sandbox_cx` | 62 | **87** |

**41 of the 43 fixes are mutation-verified** — mutation applied, suite re-run,
failing assertion named, tree restored. The two that are not (L5, L9) are
log-only and their mutants survive; the audit document says so rather than
counting them.

**A green suite is not evidence and this is the standing example.** 4,372
assertions and 100% reference coverage coexisted with three live crashes, two
fail-open divergences, and five security controls that could be deleted without
a single failure. Reference coverage counts whether a symbol is *named* by a
test.

**The whole tree is green, verified two ways on 2026-08-05.** `cyrius tests
tests` completes and reports `66 passed, 0 failed` (suites, not assertions —
see the counting note below), summing to **5,559 assertions passed, 0 failed,
0 suites unclean**.

⚠ **A note here previously said the full run "exceeds ~570 s and stalls in
`server_sse`". It does not stall** — it completes, well inside a 25-minute
bound, and `server_sse` passes **56 assertions in seconds** when built and run
on its own. It *is* slow, so the per-suite loop is still the faster way to work:
`cyrius build tests/<name>.tcyr <out> && <out>`. The eight sandbox suites total
~75 s that way.

Coverage `cyrius coverage --min 80` → **100% (1099/1099 fns)**. The denominator
counts public symbols only — `_`-prefixed internals are excluded by design,
which is why adding six of them moved it by one.

```sh
cyrius tests tests          # 66 suites; each prints "N passed" with an "(N total)" suffix
cyrius coverage --min 80    # the gate, and its own CI step
```

**There is deliberately no per-suite table here any more.** One existed and it
drifted — six rows were wrong at the 2026-07-30 refresh, and the handoff note that
caught it said to *regenerate from command output rather than editing rows by
hand*. A table that can only be maintained by hand will drift again, so the two
commands above are the authority. Counting gotcha if you sum by hand:
`cyrius tests tests` prints one line **per suite** carrying an `(N total)` suffix,
plus a final `66 passed, 0 failed` line that counts **suites, not assertions** —
sum only the suffixed lines.

Corpus size: **1,125,915 bytes** of `.tcyr`. `cyrius coverage` had a fixed 1 MiB
corpus buffer that silently under-reported past it; fixed upstream in **6.5.8**,
so there is no corpus ceiling to work around and no local coverage script.
Re-verified on 6.5.9 by the filing's own repro — padded to 1,765,916 bytes, well
past the 1,376,773 that used to report 85% and 64/75 files, it still reads
75/75 and 100%.

### Test-design decisions worth not re-deriving

The Cyrius suites deliberately exceed the oracle's coverage: they pin the UCB1
formula itself, the `max_by` last-wins tie rule, replay's zero-priority and NaN
fallback branches, and the Q-table's packed-key distinctness — none of which the
Rust tests reach.

**Three seams exist so an offline test can reach a path that needs a network.**
They are a pattern, not one-offs, and the third one confirmed it:

| module | seam | what it makes reachable |
|---|---|---|
| `tools/agnos.cyr` | transport fn pointer on the client (`:67`) | URL construction, query encoding, the path-traversal guards, response reshaping |
| `tools/wasm_tool.cyr` | `agnosai_wasm_tool_output_of` split out of `execute` | the whole result ladder, with `wasmtime` absent |
| `llm/inference_queue.cyr` | `run_item_with(item, client, chat_fp)` | the **success** arm — see below |

⚠ **The inference-queue seam exists because a mutant survived without it.** With
no gateway listening only the failure arm is reachable, and on that arm the
response is unused — so a mutant passing 0 to `settle` instead of the response
passed every assertion. That is the class of defect these seams exist to catch:
not "the code is wrong" but "no test can tell." Nothing is injected in
production in any of the three.

**A threaded test must wait on a CAUSE, not a duration.** `server_routes_sse`
drives the streaming handler on its own thread and ends it by removing the crew
from the bus; the handoff waits for the sender's **receiver count** to reach 2
rather than sleeping. A sleep races in the direction that hides the bug — tear
the crew down too early and the handler takes the unknown-crew path, where every
assertion still has something to match on the wrong frame. Same reason
`llm_inference_queue` gives the worker an `EXITED` flag: `_stopped` only reads
back what `_stop` wrote, so a worker ignoring the flag entirely passed until the
thread's own acknowledgement was observable.

`tools_builtin_load_testing.tcyr` could not follow its oracle's shape. Both Rust
tests stand up an axum mock server on loopback, and `agnosai_is_safe_url` correctly
refuses loopback — so the tool cannot be aimed at one, and pointing a test suite at
a public host is not acceptable. The **real OS-thread fan-out** runs against a
synthetic executor instead: worker threads, per-worker arenas, deadline and budget
loops, status aggregation, the sort and the percentile indices, no network. The one
path left untested is sandhi's behaviour under `sandhi_http_get_a`, covered
separately by the live `scripts/stack.sh check`.

`tools_builtin_security_audit.tcyr` solves it the same way and pays off better:
because the module is split at the network boundary, all five of its mock-server
oracle tests port exactly — `_t_mock_headers` transcribes
`mock_audit_server(security_headers, cors_wildcard)` down to the `Apache/2.4.99` it
always sets. It then goes past the oracle: both sides of every risk-band boundary,
the reflected-origin CORS bypass the probe origin exists to catch, the `to_str()`
visible-ASCII gate, case-insensitive scheme handling, and a snapshot-survives-reset
test that scribbles over the released arena so a borrowed pointer shows up as
corruption rather than passing silently.

`tools_agnos.tcyr` is where the transport seam pays off most. The oracle's three
suites test **only** names, descriptions and schemas, because every execute path
needs a live service. Since `agnosai_agnos_client_new` takes its transport as a
function pointer, a recording stub turns the whole untested half into ordinary
assertions: URL construction, form encoding, body construction, path-traversal
guards, response reshaping.

**Allocator assertions are a first-class test category now.** Every module threaded
for arenas carries an `alloc_used()`-delta assertion (0 bytes over N iterations)
plus a byte-identical-wire check against its global twin. Both are
**mutation-verified** — un-threading a single call fails them. Treat that pair as
the contract for any further threading work.

**Four things that made an arena assertion look stronger than it was.** All four
were caught by mutation, and all four generalise to any further threading:

1. **An empty fixture cannot tell a threaded route from an un-threaded one.** An
   empty collection allocates almost nothing either way. `/api/v1/dashboard/agents`
   is the sharp case: it emits an object only for a result whose metadata *names*
   an agent, so a fixture of eight crews with no agent metadata renders `[]` and
   times its guard.
2. **A route's failure arms are separately threadable and separately
   forgettable.** Measuring only the success path let `route_error_a` revert to
   `route_error` inside `agnosai_route_get_crew_a` with the whole suite green.
   The 404 is also the arm an unauthenticated scan hits hardest.
3. **Round-number thresholds assert less than they look like they do.** 128 B sat
   above a 32 B baseline and below the 176 B of the smallest un-threading
   mutation — and still passed one, because a single hoisted `str_from` costs
   16 B and lands at 48. Prefer a bound **measured in the same run**:
   `agnosai_route_resolve`'s own cost is what the arena path should equal, so
   asserting against it is self-calibrating across allocator changes.
4. **Ratios stop discriminating once the numerator is near zero.** At 4,160 → 32
   every plausible mutation still clears 4x. Absolute equality against a measured
   residual is the stronger statement and the one that is actually true: the
   handler half is zero, not merely small.

### What a crew run allocates

Per **four-task** crew run, no audit chain, no event subscribers:

| part | bytes | whose |
|---|---|---|
| building the spec (crew + 4 tasks + agent + vecs) | 4,472 | the **caller's** — in production, the request parse |
| `orch_crew_runner`'s own | **6,088** | this module |

Within `crew_runner`'s own: `agnosai_execute_task` ~528 B/task, crew state plus
four results ~200 B, remainder is the profile and metrics. No single remaining
target large enough to be worth a bite.

With a chain attached the audit path adds ~7 KB per four-task run (was ~18 KB
before the hashing scratch).

### JSON keys are hoisted, and pointer identity is what pins it

`AGN_JK_*` in `core/crew.cyr` and `core/task.cyr` hold the fifteen serialiser
keys as process-lifetime `Str`s over static literals. A `str_from_a` per key per
object costs ~32 ns and a 16-byte header; measured, four sets ran 430 ns with
keys allocated against 314 hoisted, and `agnosai_crew_state_to_value_a` went
**1,661 → 1,216 ns**.

⚠ Distinct from the constant-*return* hoist this file declines (149 sites, 2.5%
of bytes, 121 symbols). Fifteen keys, measured in time, on the paths every
serialising route runs.

**The assertion is pointer identity, not a byte bound.** A single re-allocated
key is 16 bytes and disappears inside any usable threshold — a 1,600 B bound
over a 1,488 B baseline caught a wholesale regression and missed all three
single-key mutations. `bayan_json_v_obj_key(v, i) == AGN_JK_*` is exact and
needs no calibration.

### In-loop `str_from`

Two classes, and the distinction matters: a bare `str_from` in a loop costs 16 B
on the no-free global bump per iteration; `str_from_a` costs arena bytes that the
next `reset_via` reclaims. 27 bare ones were found and **24 fixed**; 19
arena-backed ones remain and are not worth chasing.

⚠ **Three bare ones stay on purpose** — `sandbox/oci.cyr:250-251` and
`crew_runner.cyr:990` are inside loops but on `return` paths, so they run at most
once and hoisting would move an allocation onto the hot path. A naive
"`str_from` in a loop" scan will keep reporting them.

### Harness rules

- Shared helpers live in `tests/test_helpers.cyr`, all `_t_`-prefixed, so they can
  never shadow a `src/` symbol and stay out of the coverage denominator.
- Every `.tcyr` ends `var f = main(); if (f > 0) { f = 1; } syscall(60, f);` — the
  stock `proj-tcyr` epilogue masks the exit code `& 0xFF`, so exactly 256/512/768
  failures would score PASS.
- **A crash prints no assertion output.** A suite that dies mid-run reports as a
  failed *suite* with no `FAIL:` line — if a suite count drops with no failing
  assertion named, look for a segfault, not a bad expectation.
- Never `include` a stdlib module in a `.tcyr`; the stdlib is auto-prepended and an
  explicit include lands after it, single-passing into undefined symbols.
- Include order inside a `.tcyr` is load-bearing, callees before callers — e.g.
  `src/server/prometheus.cyr` must precede `src/orchestrator/crew_runner.cyr`.

Rust oracle for comparison: **863 unit + 2 integration + 1 doctest, all passing.**

## Benchmarks

**7 `.bcyr` files, 79 benchmarks, all compiling as of 2026-08-10** — see the
Tests section for how three of them had stopped. `benches/llm.bcyr` is new and
was the first to touch `src/llm/` at all.

### majra's `pq_dequeue` is O(n), so draining its priority queue is O(n²)

Found by `benches/llm.bcyr`, which is the argument for writing the benchmark
before assuming the dependency is fine. `pq_dequeue` pops with
`vec_remove(tier, 0)`, shifting the whole tail. Mean cost of one pop while
draining a tier of that depth:

| depth | per pop | ratio |
|---|---|---|
| 2,000 | 2.00 µs | — |
| 4,000 | 4.02 µs | **2.01×** |
| 8,000 | 7.92 µs | **1.97×** |
| 16,000 | 15.56 µs | **1.96×** |

Doubling the depth doubles the per-pop cost, which rules out cache effects. At
200,000 queued the mean pop was **198.7 µs** and the drain took ~40 s of memmove.

⚠ **This is the design case, not a pathological one** — `llm/inference_queue`
exists so background work is *allowed* to accumulate behind interactive work.
Filed upstream. `benches/llm.bcyr` measures the drain at **two** depths so the
slope stays legible here; if majra fixes it, the two rows converge, which is a
better regression signal than either number alone.

### The route-latency floor is `alloc_via` — measured 2026-08-07, largely fixed the same day

Do not re-derive this. Every threaded route's remaining cost is dominated by the
**number of allocations**, and each one costs an `alloc_via`.

On **6.5.9** that was **15.1 ns** — measured against a 16 ns `reset_via` control,
ten per iteration. `arena_alloc`'s fast path is ~8 instructions, so that was not
the bump; it was the five-call chain above it. Filed upstream, and **6.5.10
shipped both suggested fixes**: `alloc_via` now inlines its two accessor loads,
and `arena_allocator` registers `&arena_alloc` / `&arena_reset` directly instead
of the `_arena_*` trampolines.

**On 6.5.10 `alloc_via` is 11.1 ns** (−26%), `reset_via` 12. The remainder is
inherent to a vtable and not worth chasing: the hand-inlined
`fncall2(load64(a), load64(a+32), size)` measures 8.9 and `arena_alloc(state,
size)` called directly measures 6.2, so what is left is `alloc_via`'s own frame
plus one indirect call. **Treat ~11 ns per allocation as the floor.**

Counted exactly with a counting allocator wrapped around the arena's vtable:

| route | allocations | `alloc_via` share of latency (6.5.10) |
|---|---|---|
| `GET /api/v1/dashboard/crews` | 112 | 1,243 ns of 4,656 — **27%** |
| `GET /api/v1/crews/{id}` | 59 | 655 ns of 2,245 — **29%** |
| `POST /mcp` `tools/list` | 40 | 444 ns of 2,486 — 18% |
| `agnosai_route_resolve_a` alone | 3 | 33 ns |

Those counts are **post-hoist** and now structural: `bayan_json_v_obj_new_a` is
three allocations on its own (node, vec struct, vec data) and each pair is two
more, so a four-field object is eleven before any nesting. The response shapes
are parity-fixed against the oracle, so the count cannot fall further without an
ADR. With the upstream fix now landed, **there is no large lever left on these
routes** — the remaining cost is the object graph the wire format requires.

Two things that were measured and are *not* levers, so they need not be retried:
the dispatch shell between `route_resolve_a` and the handler is **110 ns** (A/B
in one arena: full dispatch 2,595 vs. resolve-plus-handler 2,485), and
`bayan_json_v_obj_set_a` is already minimal — pair plus `vec_push`, no duplicate
scan, and `vec_new_a` pre-sizes to capacity 16 so pushes never realloc.

`benches/learning.bcyr` — the 10 shapes of `rust-old/benches/learning.rs`. First Cyrius numbers
(x86_64 Linux, cyrius 6.5.2):

| Benchmark | Time |
|---|---|
| `capability_scorer_confidence_50caps` | 81 ns |
| `ucb1_select_10arms` | 250 ns |
| `capability_scorer_record_50caps` | 284 ns |
| `profile_success_rate_20agents` | 337 ns |
| `ucb1_select_50arms` | 1.13 µs |
| `qlearner_best_action_1000_state_actions` | 1.27 µs |
| `profile_record_20agents` | 1.84 µs |
| `qlearner_update_1000_state_actions` | 1.92 µs |
| `replay_buffer_push_1000cap_full` | 7.18 µs |
| `replay_buffer_sample32_from1000` | 583 µs |

The two replay figures are the oracle's own algorithmic cost, not a port regression: `push` runs
an O(n) lowest-priority scan on every insert once full, and `sample` recomputes the remaining
priority sum each round — which `replay.rs:90` does deliberately, "to avoid bias". Changing
either would diverge from parity and needs an ADR first.

`benches/core.bcyr` — no oracle bench file exists for core, so these measure the paths core
actually gets hammered on: id minting/rendering and the JSON round trips at every request
boundary.

| Benchmark | Time |
|---|---|
| `hw_inventory_satisfies` | 165 ns |
| `uuid_to_str` | 217 ns |
| `uuid_parse` | 252 ns |
| `uuid_v4_generate` | 526 ns |
| `message_to_json` | 8.67 µs |
| `task_from_json` | 10.1 µs |
| `agent_from_json` | 10.2 µs |
| `task_to_json` | 14.7 µs |
| `crew_to_json_10x10` | 305 µs |
| `crew_from_json_10x10` | 710 µs |

`uuid_v4_generate` is dominated by the `random_bytes` getrandom syscall, which is the right
trade: v4 ids must come from kernel entropy, never tyche. If id minting ever shows up hot, the
fix is batching entropy, not changing the source.

`benches/order.bcyr` — the numbers that settled **port plan blocker #8**, and
then retired the code that settled it. The plan measured an O(n^2) insertion sort
over agnosai's 100k percentile vector at **52.6 s**. `src/order.cyr` first
vendored heapsort + quickselect to fix that; at cyrius **6.5.4** `vec_sort_by` /
`vec_select_nth` shipped (closing agnosai's own filing) and the module became a
thin wrapper. Both transitions, measured on this box:

| Benchmark | plan baseline | vendored | stdlib (current) |
|---|---|---|---|
| `sort_100k` | 52.6 s | 79.6 ms | **20.3 ms** |
| `sort_10k` | — | 6.30 ms | **1.71 ms** |
| `sort_100k_already_sorted` | — | 79.1 ms | **3.31 ms** |
| `three_percentiles_100k` | ~21 ms predicted | 10.7 ms | **7.81 ms** |
| `select_nth_100k` | — | 6.89 ms | **5.09 ms** |
| `select_nth_100k_already_sorted` | — | 4.29 ms | **3.77 ms** |

Against the 52.6 s baseline that is **~2,600x** for the full sort. The
already-sorted row moved 23.9x in the last step alone and is a difference in
kind: heapsort's worst case equals its average so it had no fast path, while
introsort checks for pre-sorted input first. A latency vector that arrives
roughly ordered now costs a scan.

`src/order.cyr` is **98 lines, down from 184** — the vendored algorithms are
gone. What did NOT delegate is the bounds contract: `vec_select_nth` aborts the
process on `k < 0` or `k >= len`, where `agnosai_select_nth` returns 0, pinned by
three assertions in `tests/order.tcyr`. An empty latency vector is an ordinary
state for a load test that recorded no samples.

`benches/tools.bcyr` — the tool path, principally the aggregation that runs after the
`load_testing` worker threads join. `order.bcyr` already covers the sort and the selects in
isolation; what is measured here is what load_testing adds on top.

| Benchmark | Time |
|---|---|
| `tool_registry_get` | 115 ns |
| `tool_execute_echo` | 973 ns |
| `is_safe_url_public_host` | 1.30 µs |
| `is_safe_url_octal_host` | 1.38 µs |
| `lt_result_to_value` | 2.80 µs |
| `lt_aggregate_100k_10workers` | 79.2 ms |
| `lt_aggregate_100k_500workers` | 81.5 ms |
| `lt_aggregate_100k_200codes` | 149 ms |

Two design choices were on trial and both held:

- **The cross-worker merge is O(total), not O(workers).** Spreading the same 100k samples over
  500 workers instead of 10 costs 2.3 ms more — within noise of the 74.6 ms sort that dominates
  both. Each worker owns an arena freed with it, so aggregate copies rather than aliases; that
  copy is ~5 ms at the cap.
- **Status counts as a linear pair vec, not a map.** There is no `map_u64_keys` in the stdlib,
  so codes live in a vec of `[code, count]` pairs. At 200 distinct codes the scan costs 1.9x —
  but real HTTP has ~60 defined codes and a real run sees one to five, so the vec is the right
  call and the 149 ms row is the documented ceiling rather than an expected cost.

The two `is_safe_url` rows matter because every load test pays one before it touches the
network: the octal-host path, which must parse four different spellings before it can decide,
costs only 6% more than the plain hostname path.

`benches/orch.bcyr` — the orchestration path: the event fan-out on every task boundary, the
per-task ranking and delegation, the scheduler's DAG sort, the plan-cache key, and the
conversation trim that runs on every push. Logging is turned down to `SK_FATAL`, since
`agnosai_delegate_tasks` emits an INFO line per call (matching the oracle's `info!`) and left at
the default level the number measures sakshi writing to a pipe.

| Benchmark | Time |
|---|---|
| `conv_buffer_push_sliding_32` | 120 ns |
| `pattern_match_wildcard` | 803 ns |
| `event_round_trip_1_sub` | 908 ns |
| `plan_cache_get_hit` | 1.72 µs |
| `event_send_evicting` | 933 ns |
| `pubsub_publish_4_patterns` | 4.50 µs |
| `plan_key_16x16` | 11.7 µs |
| `rank_agents_16` | 12.8 µs |
| `kahn_sort_64_nodes` | 57.6 µs |
| `event_fanout_64_subs` | 53.40 µs |
| `delegate_16_tasks_16_agents` | 204 µs |
| `durable_deserialize_crew_state` | 2.68 µs |
| `durable_load_miss` | 2.80 µs |
| `durable_serialize_crew_state` | 3.88 µs |
| `durable_load_hit` | 4.43 µs |
| `durable_mkdir_p_existing_4deep` | 6.04 µs |
| `durable_save_atomic` | 21.5 µs |
| `crew_select_model_routed` | 301 ns |
| `crew_infer_provider_fallthrough` | 594 ns |
| `crew_build_system_prompt` | 1.47 µs |
| `prompt_scan_clean_67b` | 8.04 µs |
| `prompt_scan_clean_4k` | 273 µs |
| `audit_record` | 29.5 µs |
| `audit_verify_256` | 2.65 ms |
| `output_scan_clean` | 6.98 µs |
| `output_redact_clean` | 9.69 µs |
| `metrics_record_task` | 5 ns |
| `metrics_gather` | 7.94 µs |
| `auth_check_disabled` | 6 ns |
| `auth_check_secret_short_token` | 942 ns |
| `auth_check_secret_ok` | 980 ns |
| `auth_check_secret_reject` | 961 ns |
| `auth_jwt_reject_bad_alg` | **1.201 ms** |
| `auth_jwt_key_prepare` | 10.5 µs |
| `auth_jwt_verify_ok` | **1.202 ms** |

The three `auth_check_secret_*` rows are the evidence for
[ADR 009](../adr/009-auth-constant-time-secret-compare.md), not padding. They sit within
**4%** of each other — accept 980 ns, reject 961 ns, and a 1-byte token against the same
9-byte secret 942 ns. The oracle's `max(a.len(), b.len())` loop would have made that last
row track the *secret's* length, which is the timing leak the ADR closes; the residual
~28 ns spread tracks the **token's** length, which the attacker chose and already knows.
`auth_check_disabled` at 6 ns is the first branch, and it is what every deployment that
has not configured auth pays per request. **That row is also why
`agnosai_auth_check` reads the clock only on the JWT path**: the obvious spelling,
`check_at(..., clock_epoch_secs())`, reads it before the config is inspected, and the bench
caught that turning this row into 1.35 µs and `auth_check_secret_ok` into 2.35 µs — both
paths where `now` is never looked at.

**`auth_jwt_verify_ok` was a sigil finding, and sigil fixed it.** It measured
3.31 ms while `lib/sigil.cyr` was 3.12.1-era, with the raw
`rsa_pkcs1v15_verify_sha256` at 3.29 ms — the port's own parsing, base64 and JSON
work was the remaining ~20 µs. sigil's "rsa repairs" (filed by agnosai as
`2026-07-30-rsa-verify-uses-secret-exponent-ladder.md`, archived upstream) removed
a constant-time secret-exponent ladder from the *public verify* path, where the
call site's own comment said it was unnecessary. Re-measured on 3.12.2:

| | 3.12.1 era | 3.12.2 (current) |
|---|---|---|
| `auth_jwt_verify_ok` | 3.31 ms | **1.202 ms** (2.75x) |
| `auth_jwt_reject_bad_alg` | 3.29 ms | **1.201 ms** |

Per-core JWT ceiling accordingly moves **~300/sec → ~830/sec**. A residual gap
against OpenSSL (14 µs for the same RSA-2048 verify on this box) remains and is
the inherent portable-bignum cost, not a further defect.

**`reject_bad_alg` still costs the same as a valid verify, and that is deliberate.**
The `alg` check sits *after* signature verification, so a rejected token pays the
modexp. The cheaper ordering was measured and rejected: checking `alg` first
rejects in 3.75 µs but parses attacker-controlled JSON before authenticating, and
`bayan_json_v_parse_buf` allocates on the no-free global bump — **62,248 bytes per
rejected request, ~53x amplification**, no credential required. Permanent memory
exhaustion is worse than recoverable CPU exhaustion. The answer to a flood is
`rate_limit`, which is ported but **not mounted** (roadmap D1, a human decision).

**These numbers were futex-bound and the stdlib fix landed in cyrius 6.5.9.**
`lib/sync.cyr`'s two-state mutex called `FUTEX_WAKE` on every release: 394 ns
per uncontended pair, now **48 ns** with the three-state lock. agnosai changed
nothing and gained 15–77% across every lock-bound benchmark — `tool_registry_get`
512 → 115 ns, `event_round_trip_1_sub` 1,982 → 908, `event_fanout_64_subs`
100.6 → 53.4 µs, every route arm 15–37%. The tables above are the post-fix
baseline.

⚠ The `chan_*` figures below predate the bump and are due a re-measure.

⚠ Both figures more than halved at the 6.5.9 mutex fix and the *gap* between them
closed with them — at 6.5.8 they were 2.40 µs against 1.99 µs. The conclusion is
unchanged and now holds with more margin, but the arithmetic behind it was
mutex-dominated and should not be quoted from the old numbers.

`delegate_16_tasks_16_agents` at 204 µs is 16 × `rank_agents_16` (12.8 µs) plus change, which is
the expected shape — hierarchical mode ranks every task independently, with no memoisation
across tasks. That is the oracle's behaviour and the reason the cost is linear in tasks × agents.

`benches/server.bcyr` — the HTTP request path, added 2026-08-05. No oracle bench
file exists: the Rust request path is axum's, and timing it would have measured
tokio. The port's is `agnosai_route_dispatch`, ordinary synchronous code.

Each threaded read route is timed on **both** arms, because threading is not free
by construction — every `_a` call carries an extra argument and `alloc_via` is one
indirection past `alloc`. The paired rows are what makes "the arena bought the
memory without costing latency" a measurement rather than an assertion. Fixture:
8 finished crews carrying agent metadata, 8 pending approvals, 1 definition, 1
registered tool. `reset_via` is **inside** the timed loop, since a sandhi worker
pays it per request.

| Benchmark | global | arena | Δ |
|---|---|---|---|
| `route_approvals_*` | 1.543 µs | **1.300 µs** | -15.7% |
| `route_tools_*` | 2.444 µs | **1.659 µs** | -32.1% |
| `route_definitions_*` | 2.674 µs | **1.997 µs** | -25.3% |
| `route_get_crew_*` | 4.231 µs | **3.126 µs** | -26.1% |
| `route_dashboard_crews_*` | 8.859 µs | **6.887 µs** | -22.3% |
| `route_dashboard_agents_*` | 11.914 µs | **9.696 µs** | -18.6% |
| `route_approvals_post_*` | 3.096 µs | **2.580 µs** | -16.7% |
| `route_mcp_*` | 4.861 µs | **3.316 µs** | -31.8% |
| `route_resolve` | 325 ns | — | in both |

The arena arm wins on all six because the global allocator is a **no-free bump**:
a request's garbage is never reclaimed, so a long-lived process walks an
ever-growing heap and loses the locality a reset arena keeps.

The other half of the same claim is memory, asserted rather than benchmarked —
`tests/server_serve.tcyr` measures it over the same fixture, bytes charged to the
global bump per request:

| Route | global | arena |
|---|---|---|
| `GET /api/v1/dashboard/agents` | 4,368 | **0** |
| `GET /api/v1/dashboard/crews` | 4,160 | **0** |
| `GET /api/v1/crews/{id}` | 2,352 | **0** |
| `GET /api/v1/tools` | 1,720 | **0** |
| `GET /api/v1/approvals` | 576 | **0** |
| `GET /api/v1/agents/definitions` | 384 | **0** |
| `GET /api/v1/crews/{unknown}` (404) | 320 | **0** |

**Every GET read route charges the global bump literally nothing.** Handler,
router, id validation, tool schemas and the response struct all land in the
request arena and are freed by one `reset_via`.

### Arena exhaustion: the request arena spills

An exhausted arena returns 0, and a `Str` of 0 is indistinguishable from a valid
one — there is no option type or error channel through the `_a` families, so the
0 flows on and the next deref faults. The primitives are fine
(`arena_alloc` → 0, `str_from_a` → 0, `vec_push_a` → -1); nothing above them can
practically check.

**The request arena is therefore set to `ARENA_FULL_SPILL`** in
`server/serve.cyr` — overflow goes to the global bump instead of faulting.
Spilled bytes are not reclaimed, which is exactly the pre-threading cost applied
to the overflow alone.

**SPILL, not GROW, and the test pins it.** GROW retains chunks across
`arena_reset`, so each of 100 workers would hold its worst-ever request forever.
`server_serve` asserts `arena_capacity_total` is unchanged as well as that the
arena filled, so swapping the policy fails.

cyrius 6.5.9 added the policy (`ARENA_FULL_NULL`/`GROW`/`SPILL`/`ABORT`) in
answer to an agnosai filing; there is no local wrapper any more.

### The write routes — every one threaded, three keep a floor

| write route | B/req global | B/req arena | retains parse-tree data? |
|---|---|---|---|
| `POST /mcp` | 3,224 | **0** | no |
| `POST /api/v1/approvals` | 1,352 | **0** | no |
| `POST /api/v1/agents/definitions` | 2,280 | **392** | yes, cloned |
| `POST /api/v1/crews` | 21,184 | **17,048** | yes, cloned |
| `POST /api/v1/a2a/receive` | 17,192 | **15,624** | yes, cloned |

Every residual is **retained state, not garbage**: 392 B is the stored agent
definition; the crew figures are execution results held in the orchestrator
registry.

**The ownership rule, which is the durable part.** A deserialiser that feeds
process-lifetime state must **clone**, not borrow, or a threaded parse leaves the
stored object pointing into a reclaimed request body — corruption, not a crash.
`agnosai_agent_from_value_a`, `agnosai_crew_req_from_value_a`,
`agnosai_task_req_from_value_a` and `agnosai_a2a_req_from_value_a` all take an
allocator and clone into it; the routes pass `default_alloc()` for whatever is
retained and the request arena for everything else.

**What actually retains, on the crew path**: the **audit chain** (the crew name
is an audit *message*) and the **task result** (the description becomes its
output, `crew_runner.cyr:460`). `CrewState` itself holds only a minted UUID, a
status, results and a profile — asserting on `crew_state_crew_id` pins nothing.

`agnosai_audit_record` clones `event`, `level`, `message`, `provider`, `model`.
⚠ `metadata` is stored **by reference** — bayan has no deep-copy primitive — so
callers must pass a tree that outlives the chain.

⚠ `agnosai_crew_from_value` is *not* on any request path: it deserialises a
persisted crew, needs an `id` no request carries, and has no caller in `src/`.

## Dependencies

**stdlib** (44 declared, now including `unicode`, order-sensitive — rationale in [`cyrius-port-plan.md`](cyrius-port-plan.md)):
base substrate · general utilities · bayan · patra · concurrency+crypto floor ·
dynamic-link floor · async · net/http/tls/ws/sakshi/sandhi

**git deps** (declare-ahead pattern, read from `cyrius.cyml` 2026-08-05):
sigil 3.12.2 · **bote 3.3.0** · majra 2.5.3 · **kavach 3.11.7** · ai-hwaccel 2.3.16 ·
tyche 1.0.0.

**kavach 3.11.7 verified end to end, 2026-08-05** — the check `state.md` itself
prescribes, run in all three places rather than assumed:

| bundle | sha256 (16) |
|---|---|
| `git show 3.11.7:dist/kavach.cyr` | `e959d81aa2b370f0` |
| kavach worktree / `origin/main` (`6567a65`) | `e959d81aa2b370f0` |
| `agnosai/cyrius.lock` | `e959d81aa2b370f0` |

Remote tag confirmed at `6567a65` through the GitHub API, so the local `path =
"../kavach"` override and a tag-only CI resolution produce the same bytes —
which is the condition the *Pins must name what is actually built* rule exists
to enforce, satisfied rather than merely intended.

`kavach_bridge` requires 3.11.7: `exec_result_set_stdout_n` does not exist
before it, so an older pin does not compile rather than silently under-scanning
— see roadmap C2.

**All six pins are now the newest upstream tag**, confirmed against the GitHub API
rather than a local `git fetch` — the dep remotes are SSH (`git@github.com:`) and a
keyless fetch fails *silently enough to look like "no new tags"*. Use
`curl -sf https://api.github.com/repos/MacCracken/<dep>/tags` to check.

> **bote 3.3.0 is pinned as of 2026-08-03** — it adds
> `dispatcher_set_server_info`, which removes the `"serverInfo":{"name":"bote"}`
> hardcode that stopped any consumer from using bote's MCP dispatcher without
> misidentifying itself. This unblocks roadmap B1's *next* surface; it does **not**
> mean `routes/mcp.cyr` should delegate to the dispatcher today, because the oracle
> uses bote's protocol types and explicitly declines its Dispatcher
> (`rust-old/src/server/routes/mcp.rs:3-5`), so hand-building the envelope IS the
> parity behaviour.
The hoosh seam targets **hoosh 2.6.0** — `usage.cost_micro_usd`, `usage.provider` and
`X-Hoosh-Cache` are read when present, and an older gateway degrades to an absent cost
rather than a fabricated one.
libro 2.8.4 arrives transitively via bote.

## Known issues in the current build

0. **⚠ A private key was committed at the repo root — `probe_key_tmp.pem`.
   Deleted from the tree 2026-08-10; STILL IN HISTORY.**

   1,704 bytes, `-----BEGIN PRIVATE KEY-----`, tracked, committed in
   `bb76e67 "errors and jwt work"`, referenced by nothing — no `.cyr`, `.tcyr`,
   `.bcyr`, `.sh`, `.yml` or doc.

   **Verified unneeded before removal**, three ways: it is not the oracle's
   fixture (sha256 `36c3a7e2…` vs `a5cdfea6…` for
   `rust-old/tests/fixtures/test_rsa_private.pem`); its public half does not
   match the frozen vectors agnosai verifies against (`…AQEAlv/hFeMqWBO6…` vs
   `…AQEA5Wu/jjUwgB2e1/Bn…` in `tests/server_auth_vectors.cyr`); and both auth
   suites pass without it — **133 + 9 assertions, 0 failures**.

   ⚠ **Deleting the file does not remove the key.** It is reachable from
   `bb76e67` for anyone who clones. Whether that warrants a history rewrite is a
   maintainer decision and is **still open**. Nothing in the port needs a private
   key at all: agnosai never signs, it only verifies, and the one keypair any
   test needs is baked into `tests/server_auth_vectors.cyr` as frozen RS256
   vectors precisely so no key file has to exist.

   `.gitignore` now carries `*.pem` / `*.key` with that reasoning, since agnosai
   has **no** legitimate PEM in the tree — so a future one is always debris.

   Also noticed alongside it, both cosmetic and both non-`src`-surface work:
   README's Quick Start is still Rust-era (`cargo build`,
   `cargo run --bin agnosai-server`, `make check`) against a tree with no root
   `Cargo.toml`, and `.gitignore` is otherwise the Rust-era file — it still
   ignores `/target`, `**/*.rs.bk`, `criterion/`, `*.profraw` and
   `tarpaulin-report.*`.

1. **35 duplicate-fn warnings at build, none from `src/` — all benign today, but know the rule.**
   ⚠ **The stated rule here was WRONG and is corrected as of 2026-08-10.** It said
   "a redefinition only rebinds call sites parsed *after* it. A dep's internal
   calls therefore keep binding to its own definition." **They do not.** Measured:

   ```cyr
   fn dup_fn(): i64 { return 11; }
   fn early_caller(): i64 { return dup_fn(); }   # parsed BEFORE the redefinition
   fn dup_fn(): i64 { return 22; }
   ```

   `early_caller()` returns **22**. Every call site binds to the last body,
   wherever it was parsed. The conclusion this entry reached — that majra's
   `pubsub_subscribe` is fine alongside libro — is **still correct**, re-verified
   by probing the subscriber struct (`sub + 8 == 0` ⇒ majra's body ran). But it
   was correct for the wrong reason, and the wrong reason would license ignoring
   every future duplicate.

   **What this does mean for us:** agnosai's own modules are included last, so
   *our* calls bind to the last definition of everything. When we call a name
   that appears in more than one bundle, check which one wins. The current
   duplicates:

   - **33 from kavach** re-exporting symbols sigil also defines, with
     byte-identical bodies (no behavioural difference either way).
   - **`_sub_new`** (majra/libro) — we should never call it, it is a dep-private
     helper.
   - **`path_exists`** — `lib/kavach.cyr:2640` uses `sys_access(path, 0)` (F_OK,
     existence only); `lib/ai-hwaccel.cyr:1385` delegates to `file_exists`, which
     opens `O_RDONLY`. **ai-hwaccel's wins for our code** — it is included after
     kavach — and the two are *not* the same contract: a path that exists but is
     unreadable answers 1 under kavach's and 0 under ai-hwaccel's. Corrected
     2026-07-30; the earlier entry had this backwards on both counts.
   - ~~**`_agnosai_is_digit`**~~ — ✅ **FIXED 2026-07-31.** Hoisted into
     `src/units.cyr`; both local copies removed. **agnosai's own duplicate-fn
     count is now zero** — all 35 remaining warnings are lib-vs-lib.

   **The compiler only warns for `fn`.** A duplicate `var` or enum member is
   **silent**, which a 2026-07-31 audit found the hard way: four duplicated
   top-level constants in `src/`, three with *different* values, three of them
   struct sizes passed straight to `alloc()`. Nothing misbehaved — purely because
   each file's own `alloc()` was parsed after its own definition and before the
   redefinition — but reordering `src/main.cyr`'s includes would have silently
   under-allocated a heap struct. All renamed to module-unique names, and
   `scripts/check-symbols.sh` now gates the whole class in CI ahead of the build.
   **`grep "duplicate fn"` on the build log is not a sufficient check** and
   should not be treated as one.

2. **`cyrius bench` accepts an argument it cannot use and reports success —
   same root cause as the `cyrius build` missing-file bug.** Measured 2026-08-05
   on cyrius 6.5.6: `cyrius bench benches` (a directory), `cyrius bench
   no-such-dir` and `cyrius bench no-such-file.bcyr` each run **zero benchmarks,
   print no diagnostic, and exit 0**. Only the **no-arg** form prints the
   `=== N passed, M failed ===` summary, so a gate written any other way has
   nothing to check and passes over nothing.

   **`bench <file>` builds and runs that file as a program** — it does not match
   the argument against a benchmark registry. `cyrius bench
   tests/sandbox_policy.tcyr` runs the *test suite* and prints `102 passed`. So
   an argument that matched nothing is the already-filed `cyrius build` case
   (stdlib prepended, entry file never opened, a valid empty translation unit)
   with a run appended — the compiler output still shows the `unreachable fns`
   note, i.e. it compiled the stdlib and ran that.

   `bench` is the only command in the family that fails this way. `cyrius tests
   <file>` and `cyrius tests no-such-dir` both exit **1** with `not a
   directory: ...`; bare `fmt` and `lint` exit 1 with usage; `cyrius lint
   no-such.cyr` exits 1 with `cannot read file`. Filed upstream as
   `2026-08-05-cyrius-bench-accepts-an-unusable-argument-and-exits-0.md`,
   cross-triaged with the `build` filing.

   `scripts/bench-history.sh` calls the bare form and is therefore correct.

   **CLAUDE.md's rule about subdirectories is right, and a note here previously
   said it was stale.** No-arg discovery scans `benches/` and `tests/` at their
   **top level only**: a probe `.bcyr` planted at `benches/zzsub/` was not found
   (still `6 passed`, marker absent from the output), and one planted at the
   repo root was not found either. `benches/` is a scanned location, not a
   counter-example to the rule.

   **Unexplained on the same run**: `lt_aggregate_100k_10workers` measured
   **25.3 ms** against the 79.2 ms recorded below, with `sort_100k` unchanged at
   20.5 ms against 20.3 — so it is not the sort, and nothing in the tools path
   was touched. Recorded as an observation, not claimed as a win; the table
   below is left at its published numbers until someone reproduces the gap.

3. **The cleanliness gates were never in CI — only run by hand.** Until
   2026-07-31 `.github/workflows/ci.yml` ran the symbol check, build, and test
   and nothing else: no `fmt`, `lint`, `doc`, `vet`, `deny`, or `coverage`,
   despite CLAUDE.md specifying them at work-loop steps 2 and 6 and calling
   coverage out as "its own CI step". The drift that hid behind it was real —
   **31 undocumented public symbols** across five modules and four untracked
   lint deferrals. `scripts/check-clean.sh` now runs all five, and coverage is
   its own step after Test.

   Two traps it exists to avoid. **`cyrius fmt`, `lint` and `doc` each take a
   FILE**; written bare they print usage and exit 1, so a gate that checks only
   the exit code reads that as a failure and one that ignores it passes over
   zero files. And **`cyrius fmt <file>` without `--check` prints the formatted
   text to stdout rather than rewriting the file** — it is not an in-place
   formatter. The gate is itself mutation-tested: it was confirmed to fail on an
   undocumented symbol, a formatting violation, and an untracked deferral.
   (`cyrius vet` is the dependency-trust gate, not a code checker — an undefined
   symbol is caught by `build`, which refuses to emit.)

## Handoff

> **Refreshed 2026-08-07.** This section is a pointer, not a diary. Everything
> that is *owed* lives in [`roadmap.md`](roadmap.md) → *Owed work*, which is the
> single list; if an item is not there, it is not owed. What
> already shipped and why is in `CHANGELOG.md`. This file carries only the numbers
> and the standing rules.

### Start here

The tree is **green and idle** — nothing is half-done, and no bite is in flight.
Read in this order:

1. `CLAUDE.md` — process, principles, DO NOTs. Non-negotiable.
2. [`cyrius-port-plan.md`](cyrius-port-plan.md) — the plan of record. Its blocker
   table is a **closed reasoning archive**, not a work list; its *Corrections*
   section is the list of things already got wrong once, and re-deriving any of
   them is wasted work.
3. [`roadmap.md`](roadmap.md) → *Owed work* — the only list of what is owed.
4. This file for the numbers.

**Then run the gate before changing anything**, so a later failure is
attributable:

```sh
cyrius build src/main.cyr build/agnosai && cyrius tests tests && \
  cyrius coverage --min 80 && ./scripts/check-clean.sh && ./scripts/check-symbols.sh
```

Expected as of 2026-08-09: build clean, **79 suites / 6,389 assertions / 0
failed**, coverage **100% (1338/1338)** over 87 referenced files, cleanliness
clean (fmt 174 · lint 95 · doc 95 · vet+deny · deps 113 verified · lib snapshot
107), **2,189 symbols across 95 files**.

`cyrius tests tests` takes several minutes; for a single suite use
`cyrius build tests/<name>.tcyr /tmp/t && /tmp/t`, which is seconds.

⚠ A backgrounded sweep enumerates its suites **once, at launch**. A suite added
while it runs is silently excluded and the pass count still reads green — which
happened on 2026-08-08 (77 of 78, missing `fleet_topology`). Re-enumerate, or
re-run the sweep, after adding a `.tcyr`.

**What a next session would most usefully pick up**, in rough order of value:

> ⚠ **Rewritten 2026-08-07.** The previous version of this table ranked M8/M10
> last as *"past the parity bar rather than debt"*. That framing is retired.
>
> **The oracle is 41,163 lines, not 27,683.** 15,427 were unported when this
> banner was written; **M8 `fleet` closed 4,443 of them on 2026-08-08**, leaving
> **10,984 (26.7%)**. The 27,683 headline counts only `rust-old/src/**/*.rs`. It excludes
> `rust-old/` minus `src/` (11,998 lines — benches, supply-chain, fuzz, tests,
> examples, Makefile), `src/presets/*.json` (810 lines, `include_str!`'d **into
> the binary**), and 672 lines of Rust-era files at this repo's own root
> (`sdk/agnosai-tool-sdk`, README, CONTRIBUTING, Dockerfile).
>
> **A first draft of this banner said "~6,225 lines" — fleet + definitions +
> telemetry only. That undercounts the src figure by 24% and the true figure by
> 2.5x.** Of the 8,206 unported *src* lines it then counted, only 6,225 were in
> the unstarted groups; the rest are holes inside groups this file calls
> complete. With `fleet` closed and `telemetry/` source-complete the *src*
> remainder is **3,441**: M10 `definitions` 1,460, and **1,981 that are holes
> inside groups this file calls complete**. M9 has no oracle *source* left — only
> the OTLP exporter body, which is copied from hoosh rather than from
> `rust-old/`. The one named milestone is now the *smaller* half of the work.
>
> ⚠ **"M0–M8 complete" is false for five of the nine.** M2 (`hwaccel` half of
> `core/resource.rs`), M3 (`hwaccel` half of `llm/router.rs`, all of
> `llm/inference_queue.rs`), M4 (`tools/{python_tool,wasm_tool,wasm_loader}.rs`
> absent), M6 (`routes/definitions.rs`'s populated arm absent; `/metrics` serves
> a different registry than the oracle), M7 (`sandbox/wasm.rs` absent).

| | Why |
|---|---|
| **M10 `definitions`** (1,460 lines) | Whole, including ZIP and YAML. **Declaring `sankoch` in `cyrius.cyml` is the entire ZIP prerequisite** — 26 `zip_*` fns are already in `lib/`. bayan's YAML is already there too. Turns `/api/v1/presets` from `[]` into a real branch. |
| **M9 `telemetry`** (322 lines) | OTLP export (copy hoosh's `otlp.cyr`, 199 lines; thread-local trace context is mandatory under `run_pooled`) plus the JSON-logging + `EnvFilter` divergence, which starts as a sakshi filing — file it early so it lands while the rest is being ported. |
| The `sandbox`-gated tools | `tools/{python_tool,wasm_tool,wasm_loader}.rs`, plus the `hwaccel` half of `core/resource.cyr`. **kavach already has a wasmtime backend with `--fuel`** (`lib/kavach.cyr:9867`), so the WASM pair is portable now. |
| roadmap B2's remaining tail | `crew_runner`'s off-request-path bayan calls — the largest un-threaded allocator surface left. Everything request-reachable is done. |
| D1 / D2 | Two decisions waiting on a human, not on work. Neither blocks anything. |

⚠ **Do not** start by re-auditing performance on the request path. It was
decomposed to the floor on 2026-08-07: the remaining cost is `alloc_via` × the
allocation count, the count is what the parity-fixed wire format requires, and
the upstream half already shipped in 6.5.10. The measurements and the two
dead-end levers are recorded under *Benchmarks*.

### Where the port is

| | |
|---|---|
| Cyrius port | **29,518 lines**, 94 files, `src/` mirroring `rust-old/src/` |
| Rust oracle | **41,163 lines** at `rust-old/` — frozen. (27,683 is the `src/`-only figure this file carried until 2026-08-07; see the banner above.) |
| Milestones | **M0–M8 complete** — the server tier serves; M7 `sandbox`'s eight modules carry the 2026-08-04 audit's 43 findings all fixed (41 mutation-verified); **M8 `fleet` closed 2026-08-08** at 12/12 modules and 719 assertions. Read the ⚠ below: "complete" is per-milestone, not per-group |
| Not started | M10 `definitions`, M11 (sandbox-gated tools + hwaccel), M12 (llm residue, tokio tests, benches, non-`src`). **M8 `fleet` closed 2026-08-08**, **M9 `telemetry` closed 2026-08-10** |
| Gates | build OK · 2,189 symbols / 95 files · fmt·lint·doc·vet·deny·deps--verify·lib-snapshot clean · coverage **100% (1338/1338)** via `cyrius coverage --min 80` · 79 suites / 6,389 assertions |

### What "M8/M9/M10 not started" actually means — corrected 2026-08-07

> ⚠ **This section used to argue that only part of M9 was a real gap**, because
> `rust-old/Cargo.toml` has `default = []` and `fleet`/`definitions` are
> `full`-only. **That framing is retired.** It was self-issued here, never a user
> decision, and it survived four handoffs telling each new session that two of
> the three remaining milestones were "past the bar rather than debt". **A cargo
> feature gate is not a scope boundary.** All three are owed, in full, along with
> the `sandbox`-gated tools, the `hwaccel` half of `core/resource.cyr`,
> `genai.rs`, `inference_queue.rs`, the `#[tokio::test]` suites, `benches/` and
> the non-`src` surface.

| oracle module | lines | status |
|---|---|---|
| `fleet/` | 4,443 | owed in full — the largest unported group |
| `definitions/` | 1,460 | owed in full, **including ZIP and YAML**: `lib/sankoch.cyr` already exports 26 `zip_*` fns (it is merely undeclared in `cyrius.cyml`) and bayan already ships `bayan_yaml_parse`. Both were listed as upstream filings owed; neither was ever real. |
| `telemetry/` | 322 | owed in full — OTLP export **and** the ungated logging init |

The logging half is the one that is also a *default-build* divergence, and it is
not OTLP: the oracle's `#[cfg(not(feature = "otel"))]` arm still runs
`tracing_subscriber::fmt().with_env_filter(..agnosai=info..).json().init()`.
sakshi has neither a JSON output mode nor an env filter, so the port emits plain
text at sakshi's default level — a **stated** divergence documented at
`src/main.cyr:22-26`, where log *content* matches line for line and transport and
format do not. Closing it needs sakshi work, i.e. an upstream ask.

Also verified rather than assumed, because both read like omissions:
`tools/{python_tool,wasm_tool,wasm_loader}.rs` are `#[cfg(feature = "sandbox")]`;
and `tools/builtin/{echo,json_transform}.rs` are **not missing** — they are folded
into `src/tools/builtin/basic.cyr`, which is a layout divergence from the
mirror rule worth knowing before diffing file-against-file.

**The binary serves as of 2026-08-03**, and **drains on SIGINT/SIGTERM** as of
the 6.5.6 bump the same day. `src/main.cyr` wires the state, installs a
`signalfd` handler, and calls `agnosai_serve`. ADR
[013](../adr/013-graceful-shutdown-via-signalfd-and-stop-flag.md) supersedes
[012](../adr/012-no-graceful-shutdown-on-sandhi.md): the blocker was never the
signal helper this file and the roadmap both used to cite — it was that sandhi's
accept loop could not be made to return, which agnosai filed and sandhi 1.9.9
fixed.

### Standing rules a new session should not re-derive

1. **`src/` mirrors `rust-old/src/`.** Anything walking it must recurse —
   `find src -name '*.cyr'`, never `src/*.cyr`, which now matches nothing and
   **fails open**. `cyrius coverage` and `cyrius tests` already recurse.
2. **Verify a toolchain bump by diffing `lib/` against the snapshot, and write
   the lockfile last.** A green `cyrius lib sync --full` is not proof: on the
   bayan repo the same command reported success and left five files stale.
   `cyrius deps` re-layers git deps but does **not** refresh the stdlib — and it
   writes `cyrius.lock` from what it just copied, so a sync afterwards leaves
   `deps --verify` failing. The order that satisfies both gates:

   ```sh
   cyrius deps --no-lock     # resolve git deps into lib/
   cyrius lib sync --full    # refresh the stdlib over them
   cyrius deps --lock        # record what is actually there
   ```

   **A dep's transitive pin can hold a stdlib module back, and `lib sync` is
   not the fix.** `cyrius build` performs an implicit resolve, so a stale module
   flips back on **every build**. Diagnose by hashing — several sources can hold
   the same version and only one is the writer, so version stamps mislead:

   ```sh
   sha256sum lib/<mod>.cyr ../*/lib/<mod>.cyr \
             ~/.cyrius/deps/<mod>/*/dist/<mod>.cyr \
             ~/.cyrius/versions/<pin>/lib/<mod>.cyr
   ```

   `deps --verify` cannot catch this — the lock is written *from disk*, so a
   downgraded file gets its downgraded hash recorded and the check passes.
   `scripts/check-clean.sh`'s snapshot diff is the only gate that sees it, and
   it now allows no exceptions. The reasoning and the wrong first diagnosis are
   in [`cyrius-port-plan.md`](cyrius-port-plan.md) → *Corrections*.

   Also: `cyrius lib sync` reads `~/.cyrius/versions/<the pin>/lib`, so a
   sibling on an older `cyrius = "X.Y.Z"` merely re-syncs its own old snapshot.
   Bump the sibling's pin first, then run the three-step there. All seven
   siblings are on 6.5.10 as of 2026-08-07.
3. **Git-dep pins must move with the fold.** `cyrius deps` copies each dep's
   bundle into `lib/` last-write-wins, so a stale `[deps.sigil] tag` overwrites a
   newer folded copy back down.
4. **Cyrius silently accepts a duplicate parameter name.** Adding an allocator
   parameter named `a` to a fn whose first parameter was already `a` compiles
   clean, and every `load64(a + OFFSET)` then reads the allocator. It SIGSEGVs with
   no assertion output. Check existing parameter names before prepending.
5. **`map_keys` defeats arena threading.** It materialises a key vec through
   `vec_new()` on the no-free global bump and has no `_a` form, so it survives
   every other substitution and silently caps the win. Use `_agnosai_map_slots` /
   `_slot_live` / `_slot_key` / `_slot_val` from `src/core/json.cyr`.
6. **The duplicate-symbol hazard is wider than `fn`.** The compiler warns on a
   duplicate `fn` and is **silent** on a duplicate `var` or enum member.
   `scripts/check-symbols.sh` gates the whole class; `grep "duplicate fn"` on a
   build log is not a sufficient check.
7. **Measure before claiming, and mutation-test the assertion.** Two findings this
   session reversed on measurement (the `to_wire` hoist looked worth 2.5% and was
   not worth 121 new globals; `map_keys` looked negligible and was the entire
   residual), and two bugs were caught only because the pinning assertion was
   mutation-verified.
8. **When an assertion cannot be made to fail, that is usually a statement about
   the code's shape, not the test's.** The M7 audit remediation's recurring
   defect was an *unreachable* control, not an unasserted one — an env filter
   that reads `/proc/self/environ` cannot be tested by a process with no
   `setenv`. Five fixes worked by splitting the function so it takes its input
   as a parameter (`_agnosai_sanitized_envp_of`, `_agnosai_process_envp_from`,
   `_agnosai_cx_resolve_interpreter`, `_agnosai_cx_budget_ms`,
   `_agnosai_sandbox_manager_timeout_ms`); two more by **re-execing the suite
   through `agnosai_spawn_capture_input` with a planted `envp`**, which is the
   only way to put a variable in this process's own environment. Reach for one
   of those before concluding a behaviour is unobservable.
9. **`callptr` does not check arity, so changing a function-pointer signature is
   only half-caught by the compiler.** The tool vtable's `schema_fp` went from
   `fn(ctx)` to `fn(a, ctx)`. Implementors still declaring the one-parameter
   shape **compiled and passed** — `ctx` silently received the allocator and the
   real ctx was dropped, which is invisible in a schema that ignores `ctx`. Two
   test tools in `tools_native.tcyr` were in exactly that state and the suite
   reported 71/71.

   The *detectable* half is a direct call: `_agnosai_delegate_schema(0)` in
   `tools_builtin_delegate.tcyr` then passed 0 as the **allocator**, and
   `alloc_via` followed it — a segfault, so a failed suite with no `FAIL:` line
   (see the crash note in the harness rules).

   **Neither showed up in per-suite verification.** The change was mutation-
   tested against `server_serve` and `tools_native`, both green; only
   `cyrius tests tests` across the whole tree caught it. When a signature that
   is reached through `callptr` changes, grep for every implementor and every
   direct caller by name — the build will not.

10. **A tool that prints a number can still be lying, and the fix is upstream.**
   `cyrius coverage` under-reported for a fortnight because this tree's `.tcyr`
   corpus outgrew a fixed 1 MiB buffer — silently, at exit 0, naming
   fully-covered files as uncovered. It was worth a local reimplementation only
   until upstream shipped (6.5.8), and that reimplementation was then deleted.
   The durable half: when a gate's number moves and nothing in the diff explains
   it, **suspect the gate before the code** — and confirm by computing the same
   thing a second way rather than by reasoning about it.

11. **Verify the mutation is a faithful revert.** One written during the M7
   remediation changed half of a two-line fix, left the tree in a state neither
   version would produce, and "killed" the mutant for the wrong reason. Read
   what the mutated file actually says before believing the failure.

   **The inverse failure happened on 2026-08-08 and cost two rounds.** The
   mutation for `tests/server_auth_lane_race.tcyr` reverted the *call site* in
   `_agnosai_auth_validate_jwt`, while the suite calls
   `_agnosai_auth_rsa_verify_locked` **directly** — so the test never saw the
   change and reported 9/9 against a knowingly-broken build. **Mutate what the
   test actually calls**, not what the production path calls, and if a mutant
   survives, suspect the mutation's reach before rewriting the test. (A real
   test weakness was also found the same way: 2 threads × 200 iterations with no
   start barrier left the unlocked build fully clean, because the workers barely
   overlapped. The suite now uses a barrier and 2,000 iterations.)

12. ⚠ **A `return f(&local_or_pointer_into_this_frame, ...)` with EXACTLY SIX
   arguments is a tail call, and cycc frees the frame before jumping.** The
   callee then reads a released frame. This cost sigil three releases and cost
   this tree a wrong diagnosis, so it is worth knowing cold:

   - cycc declines TCO **above** six arguments, so the same code localised
     cleanly in a 10-argument function and "broke the tests" in a 6-argument
     one. **The arity was the variable, not the buffer.**
   - cycc's own guard fires only on a `&` spelled literally in the argument
     list, so `var p = &buf; ... return f(p)` walks straight past it.
   - Fixed in **cyrius 6.5.14**. Below that pin, the hazard is live.

   **The wrong turn, recorded because it was persuasive.** Localising sigil's
   PSS and sign buffers regressed tests; three sigil releases (and a note in
   this tree) recorded "cause not understood" and kept the shared lanes — and
   those lanes *were* an authentication bypass. This tree's probe that
   "ruled out callee-clobbers-caller" was **not a faithful reproduction**: it
   called the callee and then did more work, which is not a tail call, so TCO
   never applied and the probe returned a false negative. The probe that
   settled it compares two addresses — a callee local sitting *above* every
   caller local only happens if the caller's frame is already gone.

   **When a buffer "cannot" be localised, count the callee's arguments first.**

13. **sigil's crypto scratch is shared across threads, and the JWT path is the
   only place agnosai is protected.** `cbank()` gives 63 lanes and never
   releases one, so the bound is 63 *lifetime* crypto-touching threads — a
   count agnosai passes with its 100 pool workers alone. Both operands of
   `_rsa_pkcs1v15_check`'s final `ct_eq_bytes` live in that shared lane, which
   is an authentication bypass, not merely a spurious-401 race: **888 of 400,000
   forged signatures were accepted** before the fix. `src/server/auth.cyr`'s
   `_agnosai_auth_rsa_verify_locked` serialises the verify; **do not remove that
   mutex to reclaim throughput.** ⚠ 62 file-scope banked globals remain,
   including the shared bignum engine and the TLS 1.3 peer-auth lanes every
   outbound HTTPS call uses — those are **not** covered by agnosai's lock. Full
   analysis in `CHANGELOG.md` under *Security*; the upstream ask is in
   `roadmap.md` → C.

   ⚠ **Do not re-test this by pinning every thread to one lane.** Max contention
   shows zero false accepts and near-total false rejects, which reads as
   fail-closed and is wrong — the bypass lives in the low-multiplicity regime
   that 100 threads over 63 lanes actually produce.

### Consumers

Agnostic (Python platform), daimon (agent orchestration), joshua (NPC AI),
kiran (game AI) — none consuming the Cyrius line yet.
