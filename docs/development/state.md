# AgnosAI — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures
> (durable); this file is **state** (volatile).
> Last refreshed: 2026-07-28.

## Version

**1.1.0** (`VERSION`) — the last shipped Rust release, now preserved at
`rust-old/`. The Cyrius line targets **v2.0.0**; VERSION bumps once parity
lands, not before, so the number always names something that actually shipped.

## Toolchain

- **Cyrius pin**: `6.4.83` (`cyrius.cyml [package].cyrius`)
- Rust (for `rust-old/` only): `channel = "stable"`, currently rustc 1.96.0

## Source

- **Rust reference**: 27,683 lines at `rust-old/` — frozen, do not edit. It is the parity oracle.
- **Cyrius port**: scaffold only. `src/main.cyr` is the stub; no domain code ported yet.

## Where the port is

Phase 0 (M1 scaffold) — **dependency scaffold complete**.

| Gate | Status |
|---|---|
| Rust baseline green (the oracle) | ✅ fmt + clippy clean, all 9 feature combos compile, 863 + 2 + 1 tests pass |
| Terminal Rust benchmark capture | ✅ 112 rows at v1.1.0, frozen into `rust-old/bench-history.csv` |
| Blockers re-verified vs 6.4.83 | ✅ see [`cyrius-port-plan.md`](cyrius-port-plan.md) |
| `cyrius port` run | ✅ 2026-07-28 |
| `cyrius lib sync` + `cyrius deps` | ✅ 43/43 stdlib modules, 9 deps resolved, 0 errors, 79 locked |
| Hello-world builds and runs | ✅ `cyrius build src/main.cyr build/agnosai` → OK |
| `src/id.cyr` (uuid v4/v5) | ⬜ not started |
| `src/order.cyr` (`ai_sort` / `ai_select_nth`) | ⬜ not started |

## Tests

No Cyrius tests yet — `tests/agnosai.{tcyr,bcyr,fcyr}` are scaffold stubs.

**Before writing the first one:** replace the stock epilogue with
`var f = main(); if (f > 0) { f = 1; } syscall(60, f);` — the stock form masks
the exit code `& 0xFF`, so exactly 256/512/768 failures score PASS.

Rust oracle for comparison: **863 unit + 2 integration + 1 doctest, all passing.**

## Dependencies

**stdlib** (43 declared, order-sensitive — see the manifest's comments):
base substrate · general utilities · bayan · patra · concurrency+crypto floor ·
dynamic-link floor · async · net/http/tls/ws/sakshi/sandhi

**git deps** (declare-ahead pattern): sigil 3.12.1 · bote 3.1.4 (core profile) ·
majra 2.5.1 · kavach 3.9.3 · ai-hwaccel 2.3.15 · tyche 1.0.0.
libro 2.8.2 arrives transitively via bote.

## Known issues in the current build

1. **majra's pubsub subscribe path is unusable.** libro and majra both define
   `_sub_new` with different arity and semantics; libro wins under
   last-definition-wins, so majra's `_sub_new(ch, filter_fn)` calls bind to
   libro's `_sub_new(pattern)`. Reordering the declarations does not fix it —
   `cyrius deps` derives include order from its own graph. Not blocking today
   (nothing calls it yet); blocks M5 unless resolved. The port plan already
   recommends agnosai's own channel-backed event bus for a separate reason.
2. **35 duplicate-fn warnings at build.** 33 are kavach re-exporting symbols
   sigil also defines, with **byte-identical bodies** — harmless. The other two
   are the `_sub_new` above and `path_exists` (ai-hwaccel uses `file_exists`,
   kavach uses `sys_access`; same contract, different implementation).
3. **`cyrius.cyml` comments must not contain brackets or double quotes inside
   the stdlib array.** The 6.4.83 cyml reader does not strip comments: a
   bracketed token is read as a section header and silently truncates the array,
   and a quoted token is harvested as an entry. This cost real debugging time —
   see the CAUTION note in the manifest. Filed upstream 2026-07-28.

## Consumers

Agnostic (Python platform), daimon (agent orchestration), joshua (NPC AI),
kiran (game AI) — none consuming the Cyrius line yet.

## Next

M2 in [`roadmap.md`](roadmap.md): the `learning` beachhead. Zero coupling, zero
async, zero I/O — it exercises tyche, f64-as-bit-patterns, and the `.tcyr`
harness with no downstream risk. Then `core`.
