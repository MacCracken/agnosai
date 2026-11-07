# M12 bench audit — 2026-08-10

**What this is.** The 83 benchmarks that closed M12's bench gap were written by six
parallel agents, one per `benches/*.bcyr`, each of which compiled and ran its own file.
Every one was then handed to a separate agent told to assume it was wrong. This file is
those six adversarial reports, verbatim, with my triage on top.

**Why it is kept.** The reports are more valuable than the benchmarks. Four of them found
something real, and the two that mattered most were things the *implementing* agent had
stated confidently in a comment. A benchmark whose comment lies is worse than no benchmark,
because the comment is what the next reader reasons from.

**How to use it.** Before editing any row in `benches/`, read that file's section here.
Several comments in the shipped files are known-wrong and are listed below rather than
fixed; do not re-derive them.

---

## Triage

| file | added | verdict | action |
|---|---|---|---|
| `core.bcyr` | 9 | **clean** — "no defects found", 2 nits | none needed |
| `definitions.bcyr` | 5 | 7 findings, 1 substantive | **owed** |
| `server.bcyr` | 10 | 7 findings, "not sound" | **owed** |
| `tools.bcyr` | 8 | 7 findings, 1 non-reproducing number | **owed** |
| `fleet.bcyr` | 14 | 5 findings, 2 SEVERE | **1 fixed, 1 rejected, 3 owed** |
| `orch.bcyr` | 37 | see section | **owed** |

## What I acted on

**FIXED — `fleet.bcyr`, four placement shapes never reached the sort.** Confirmed by
reading `lib/vec.cyr:340-352`: `vec_sort_by` runs an O(n) already-ordered pre-check and
**returns before `_vec_introsort`**. The comments claimed these rows guarded the
score-only-comparator divergence at `src/fleet/placement.cyr:20-23`; with a score-only
comparator every adjacent compare returns 0, the pre-check still short-circuits, and the
timing is identical — structurally blind to the one regression they named.

⚠ **The obvious fix does not work.** Reordering the input fleet changes nothing:
`agnosai_rank_nodes` assigns `AGN_PR_INDEX = i` from the scan position
(`src/fleet/placement.cyr:283`) and pushes survivors in scan order, so output indices are
ascending **by construction** whatever order the input arrives in. Measured: a stride
shuffle gave **11.763us** against the ordered fleet's **11.722us**. The *scores* have to
vary. `fleet_rank_nodes_gpu_affinity_200_varied` uses a four-tier VRAM sawtooth and prices
the sort at **19.53us against 11.80us** — a 7.7us difference the original pair could not
see at all.

**REJECTED — `fleet.bcyr`'s `clock_epoch_secs` = 1.318us is NOT unsourced or implausible.**
The verifier called it fabricated, reasoning from `lib/bench.cyr:6`'s documented
`clock_gettime: ~120ns` and from `uuid_v4_generate` costing 501ns while making a real
`getrandom`. Careful reasoning, wrong conclusion. Measured independently at 2,000,000
iterations: **`clock_epoch_secs` 1.315us, `clock_now_ns` 1.318us**. The implementing agent
was right.

⚠ **It is `lib/bench.cyr:6`'s constant that does not hold on this host** — Cyrius issues a
raw `syscall(228, ...)` rather than taking the vDSO path, and that constant is what every
benchmark in the ecosystem implicitly subtracts. The actionable half of the verifier's
critique stands and is done: the figure is now a measured row
(`clock_epoch_secs_baseline`) so the header's arithmetic is falsifiable from
`bench-history.csv` instead of asserted in prose.

## fleet finding 3 — CLOSED 2026-08-11, and the plan's framing of it was wrong

Six of the plan's 19 gaps — the `scoring.rs` set — shipped nowhere. Now in
`benches/orch.bcyr`, all seven oracle ids.

The plan called `rank_agents_1000_varied` "THE ONE THAT MATTERS" because the port
hand-rolls an insertion sort where the oracle uses `sort_by`. **The quadratic is real and
now measured at three sizes** — 127.8 / 626.0 / 5093.0 us at 100 / 300 / 1000 agents, with
the 300 point landing within 2% of what a fit to the other two predicts, so the curve is
tested rather than assumed. Sort overtakes scoring at n ~ 205.

⚠ **But "an agent count an attacker can influence" is FALSE, and I repeated it into the
roadmap and CHANGELOG before checking.** `AGNOSAI_CREW_MAX_AGENTS` is **100**, enforced at
`src/server/routes/crews.cyr:353`. Nothing reachable over HTTP passes n = 100, where the
sort is a third of a 128 us call — well below the crossover. It is a performance note with
a documented bound, not a DoS vector. Corrected in both places.

That makes three claims this milestone that were confident, well-cited, and wrong: this
one, the `err_io` divergence, and the `clock_epoch_secs` rejection. Two were mine.

## definitions finding 2 — CLOSED 2026-08-11, and it was a real src defect

The report said `benches/definitions.bcyr:33-34` "states a src defect as an inherent
constraint". It was right, and the defect was live: `AGN_ROUTE_LIST_PRESETS` was the only
id in `_agnosai_route_dispatch_inner`'s ladder with an `_a` form that went unwired, so
`GET /api/v1/presets` parsed all 18 preset documents onto the no-free global bump on every
request. Fixed; see CHANGELOG.

⚠ **The regression test took three attempts and the first two proved nothing** — both arms
return identical bodies, and the outer dispatch allocates 32 bytes into the arena on its
own, which satisfies a naive `> before`. Wired vs unwired arena growth is **30,464 vs 32
bytes**; the shipped assertion is a threshold at 8 KiB, verified by reverting the fix and
watching it fail.

## What is owed

Everything else below.

⚠ **Do not treat these reports as authoritative either.** One of the two SEVERE findings
was wrong, and it was wrong in the confident, well-cited style that reads as reliable.
Verify against the tree before acting on any of it — which is the whole lesson of this
milestone.

### Known errors IN this document (found 2026-08-11 while remediating)

Recorded rather than edited out, because the point of keeping the reports verbatim is
that they can be checked:

- **"nine `.bcyr`"** (findings 3 and 7, definitions section) — there are **ten**. The count
  is used as the reason a dropped bench line is easy to miss, so it is load-bearing.
- **The "numbers cohere" arithmetic** (definitions) assumes **15** fields per agent; the
  serializer emits **14**, so every per-field figure derived from 1080 is wrong.
- **The `AgentDefinition` derive citation** (definitions finding 1) is wrong in both span
  and field inventory. The finding's substance — the oracle deep-clones each pick and the
  port does not — stands and was applied.
- **"the tools/list envelope is 45 bytes"** (server finding 7) — it is **46**; the file was
  right and the audit was wrong. The neighbouring `initialize` count (135) really was a
  file defect and is now 136.
- **"`_agnosai_orch_audit` fires twice per run"** (server finding 2) — **three** fire. The
  crew runner writes a `task_completed` record per task on top of the orchestrator's
  `crew_accepted` and `crew_finished`. The audit repeated the file's own count while
  correcting its neighbour.
- **"the probe resolve is a small fraction of the /api/v1 resolve"** (server finding 5) —
  it is **41–44%**. The conclusion (`route_resolve` is the wrong residual for the probe
  rows) is right and was acted on; the magnitude is not.

That is six errors in a document whose entire purpose was to find errors, which is the
honest measure of how far to trust any single reviewing pass — including this one.

---

==============================================================================
FILE: definitions.bcyr  (5 added)
==============================================================================
## Findings — `benches/definitions.bcyr`

The five measurements themselves are mechanically sound (I reproduced them three times, ±4%). The defects are in what the file *claims*, and one of those claims hides a real undocumented divergence in `src/`.

---

### 1. `benches/definitions.bcyr:159-161` — the "oracle's shape" claim is wrong by ~2×, and it papers over an undocumented port divergence

The comment under the oracle's id says:

```
# One call is 5 × 20 = 100 `match_score` evaluations plus the `max_by` scan and
# one result vec — the only allocation in the whole call, since the picks are
# borrowed pointers into `available` (assembler.cyr:164-171).
```

That describes the **port**, not the oracle. `rust-old/src/definitions/assembler.rs:34-47`:

```rust
.max_by(|a, b| match_score(member, a).partial_cmp(&match_score(member, b)) ...)
.filter(|agent| match_score(member, agent) > 0.0)
.cloned()
```

`Iterator::max_by` invokes the comparator n−1 times and the comparator calls `match_score` **twice**: 19 × 2 = 38, plus 1 more in `.filter()` → **39 per member, 195 per call**. It then deep-`.cloned()`s each of the 5 picks (`AgentDefinition` is `#[derive(Clone)]` over 4 `String`s, a `Vec<String>` and 4 `Option<String>`s — `rust-old/src/core/agent.rs:10-40`).

The port's `_agnosai_assembler_best` (`src/definitions/assembler.cyr:136-157`) evaluates `match_score` **once per agent**, caches it in `best_score`, and reuses that cached value for the `> 0.0` filter — 100 per call, zero clones.

Two things to change:
- Fix the comment so it does not present the port's count as the oracle's, and so the `n=5000 → 500k inner passes` calibration on `:163-165` is not derived from a number that only holds for the port.
- The substantive item: `_agnosai_assembler_best` memoizing where the oracle recomputes is an **undocumented divergence**. `assembler.cyr:9-31` carefully documents three subtler traps (`max_by` last-max, the `>0` filter, f64 scores) and says nothing about this one. Per CLAUDE.md ("Diverge only with an ADR") it needs a module note at minimum. Behaviour is identical — `match_score` is pure — so this is a note/ADR, not a code change.

### 2. `benches/definitions.bcyr:33-34` — states a src defect as an inherent constraint, and hands the wrong instruction to the next agent

```
# `src/server/router.cyr:567` has no arena arm for that id, so only a
# global-allocator `route_presets_global` bench is possible.
```

`agnosai_route_list_presets_a(a)` **exists** (`src/server/routes/definitions.cyr:36`), and `_agnosai_route_dispatch_inner` already has the arena `a` in scope at line 567. Every neighbouring `_a` route in that ladder carries the arm:

```
546	    if (id == AGN_ROUTE_LIST_DEFINITIONS) {
547	        if (a != 0) { return agnosai_route_list_definitions_a(a, state); }
548	        return agnosai_route_list_definitions(state);
549	    }
...
567	    if (id == AGN_ROUTE_LIST_PRESETS) { return agnosai_route_list_presets(); }
```

`AGN_ROUTE_LIST_PRESETS` is the **only** id in 525-575 with a global-only body despite having an `_a` form (`AGN_ROUTE_CANCEL_CREW`, `REMOVE_TOOL`, `A2A_STATUS` etc. have no `_a` route at all). So the arena arm is *missing*, not impossible — an arena-threading bug against CLAUDE.md's "Thread the `_a` allocator variants". Change the comment to file it as a defect, not a limitation; otherwise whoever owns `benches/server.bcyr` will write `route_presets_global` and treat the missing arm as settled.

### 3. `benches/definitions.bcyr:86-87` vs `:282-286` — the guards are advertised as loud and are silent

`:86-87` claims the `vec_len != 18` check "fails loudly if the generated file ever stops holding eighteen parseable documents — which would silently make this bench cheap." All four guards `return 1`, and `main` discards every return:

```
282	    _b_builtin_presets();
283	    _b_assemble_team();
...
288	    return 0;
```

A tripped guard drops that bench line from the report and the run still exits 0 / `=== 1 passed, 0 failed ===`. In a nine-file sweep a missing line is easy to miss. This is house style across all nine `.bcyr` (none propagate), so the minimum fix is to stop claiming otherwise in the comment; the better fix is `var f = _b_builtin_presets(); ...; return f;`.

### 4. `benches/definitions.bcyr:250-257` — the one deviation from the plan is justified by an argument the same file refutes

`preset_to_value_18` runs `n=500` instead of the plan's 2000, because "At 2000 that is ~175 MB on top of everything else, on an allocator that never frees." But `load_preset_from_json_5a` in the same file runs **20 000** full parses of the 762-byte document, each building a `PresetSpec` plus 5 `AgentDefinition`s plus the whole bayan value tree — by the comment's own per-unit pricing that is several times the 175 MB it rejects, and `builtin_presets_18` at n=1000 (18 documents, 21 295 bytes, 72 agents per iteration) is larger still. Either raise it to the planned 2000, or replace the memory rationale with the true one.

### 5. `benches/definitions.bcyr:85-86` — "first-touch warm-up" is not a thing on this allocator

"It also serves as the first-touch warm-up so heap growth is not charged to iteration 0." `lib/alloc.cyr:12` is a monotonic no-free bump over `brk`; **every** one of the 1000 iterations faults fresh pages, so one untimed call warms nothing iteration 1 wouldn't. Keep the guard (it is worth having), drop the warm-up justification.

### 6. Low — three off-by-one line citations in a file that leans on precise citations

- `:28` — `agnosai_route_list_presets_a` is `src/server/routes/definitions.cyr:36-43`, not `35-42`.
- `:255` — "bump, no individual free" is `lib/alloc.cyr:12-14`, not `13-15`.
- `:44` — packaging/`zip_*` is `src/definitions/mod.cyr:18-19`, not `19-20`.

### 7. Low, house-wide — single-batch timing makes the reported min/max tautological

All five benches call `bench_batch_start`/`stop` exactly once, so `min == max == avg` by construction (visible in every sample line). `lib/bench.cyr:200-211` documents a rounds×batch pattern that gives real variance; nothing in `benches/` uses it (verified across all 9 files, 107 `bench_batch_start` calls, zero inside a rounds loop). Not this file's defect — flagging it as a systemic gap since the numbers carry no dispersion signal.

---

## What I checked and found sound

- **`_B_PRESET_JSON_5A` is byte-faithful.** Unescaped the continued literal and `json.loads`-compared it against `rust-old/benches/definitions.rs:71-84`: structurally equal, key order identical, all 5 agents identical field-for-field. 762 B collapsed vs 961 B indented; the whitespace divergence is disclosed at `:19-22`.
- **No setup inside any timed region.** All four fixtures (`available`/`members`, the match pair, `js`, the 18 parsed presets) are built before `bench_batch_start`; only `vec_get` (18/iter) sits inside `_b_preset_to_value`'s batch.
- **Nothing is measuring nothing.** No loop body is invariant-hoistable in practice — 251-258 ns for `assembler_match_score` at 500 000 iterations proves it is not being elided, and there is no 0-2 ns line.
- **Symbols and arity all resolve as called**: `agnosai_builtin_presets()/0`, `agnosai_assemble_team/2`, `agnosai_assembler_match_score/2`, `agnosai_load_preset_from_json(Str, out_err)/2`, `agnosai_preset_to_value_a(al, p)/2`, `agnosai_team_member_new/3`, `agnosai_agent_new/3`, `agnosai_agent_with_name/2`, `agnosai_agent_with_tools/2`, `default_alloc()/0`, `bench_*`.
- **The oracle's two load-bearing quirks are reproduced correctly.** Complexity is left at `agnosai_default_complexity()` = `"medium"` (`src/core/agent.cyr:51-53`, stored at `:92`), matching `make_agent`'s discarded `_complexity`; names are `"agent-{i} Agent"` so the +2 name bonus never fires. `_b_def_agent`'s `i%5`/`i%4` tables match the oracle's arms including the `_` fallbacks.
- **Fixture facts verified against the tree**, not taken on trust: `AGNOSAI_PRESET_JSON_COUNT = 18` (`presets_data.cyr:20`); `src/presets/*.json` = 18 files, **72** agents, **21 295** collapsed bytes (the comment's "~72" and "~20 KB").
- **The numbers cohere.** `builtin_presets_18` at 41.3 µs/document vs `load_preset_from_json_5a` at 32.5 µs → 37 vs 43 ns/byte, consistent. `preset_to_value_18` at 145 µs / 1080 `obj_set` calls = 134 ns/field, consistent with the parse being ~5× the serialize. The one number that *looks* wrong — `assemble_team` at 185 ns/pair being cheaper than the isolated `assembler_match_score` at 254 ns — checks out: the isolated pair is the full-ladder one (6-byte `eq_ci` hit + 8-char tool `eq_ci` + 6-char complexity `eq_ci` ≈ 50 `_agnosai_fold_ascii` calls), while 96 of the 100 pairs in `assemble_team` bail on the length guard in `agnosai_str_eq_ci` (`src/strcase.cyr:43`) and are call-overhead-bound at ~8 calls × ~22 ns. Not a finding.
- **Nothing silently dropped vs the plan.** All 3 oracle ids plus both port-local extras (`assembler_match_score`, `preset_to_value_18`) are present; extra #3 was correctly forwarded as a header note for `benches/server.bcyr`. The two deviations (dropped `src/order.cyr` include, `n=2000→500`) are both disclosed — see finding 4 for the second.
- **The include list is correct**: narrow list over `src/definitions/mod.cyr` avoids `packaging`'s sankoch `zip_*`; the `assembler → presets_data → loader` order matches `mod.cyr:30-33`; dropping `src/order.cyr` is proven safe by the build (single-pass resolution would have errored). File sits flat in `benches/`, so no-arg `cyrius bench` discovery finds it. No stdlib include. `sakshi_set_level` correctly omitted — `loader.cyr` and `assembler.cyr` emit nothing.
- **CHANGELOG carries the new file** (`CHANGELOG.md:181`).

==============================================================================
FILE: core.bcyr  (9 added)
==============================================================================
No defects found. Two nits and one caveat, then what I verified.

## Nit 1 — wrong stated rationale for the iteration counts (benches/core.bcyr:240-242)

```
# What DOES constrain them: `bench_batch_stop` reports an INTEGER ns-per-op
# (lib/bench.cyr:223), so cheap shapes need a large batch to land on a stable
# number. Hence "high", not "equal".
```

The truncation at `lib/bench.cyr:223` is `var per_op = elapsed / batch_size;` — on the **quotient**, so the reported value is floored to a whole ns *regardless of n*. Raising the batch does not improve that resolution; it amortizes the two `now_ns()` calls and reduces timer noise, which is the actual reason. Consequence worth knowing rather than fixing: `hw_inventory_satisfies_10dev_empty_req` reports `8ns`, so it carries ±1ns ≈ 12% blindness that 1e6 iterations cannot remove — a sub-12% regression on that row is invisible. Change the comment to say "amortize timer overhead", not "land on a stable number".

## Nit 2 — 5 of 9 iteration counts silently deviate from the plan

| bench | plan | file |
|---|---|---|
| `hw_inventory_satisfies_10dev_cuda` (:261) | 300000 | 500000 |
| `hw_inventory_devices_of_type_cuda_10dev` (:296) | 300000 | 500000 |
| `hw_inventory_total_memory_mb_cuda_10dev` (:312) | 300000 | 500000 |
| `hw_inventory_to_json_8dev` (:383) | 50000 | 20000 |
| `hw_inventory_from_json_8dev` (:406) | 50000 | 20000 |

All five are defensible and I verified the in-file reasoning is *more* correct than the plan's: `vec_new_a` (lib/vec.cyr:30) allocates a 24-byte header plus a 16-slot buffer up front = 152 B, so the plan's premise ("grows 1→2→4→8, ~144 B/iter") was wrong — 8 pushes never touch the grow path, and 500000 is ~76 MB, not the plan's feared regime. Per-op-ns reporting keeps the 20000-iteration JSON rows comparable to the 50000-iteration task rows. The one constraint the plan called load-bearing — `devices_of_type` and `total_memory_mb` sharing an identical count so the delta isolates the summation — is preserved (both 500000). So: no change needed, but the report of "9 benchmarks" did not disclose that 5 counts moved.

## Caveat (pre-existing, not introduced here)

The sub-µs rows are partly page-fault-bound. Every `satisfies`/`devices_of_type` iteration leaks 152 B on the no-free global bump, i.e. a fresh 4 KiB page every ~27 iterations; at ~0.5-2 µs per minor fault that is roughly 18-74 ns of the 257 ns `devices_of_type` number. The file acknowledges this at :238-239. It affects every row in the repo equally and does not invalidate the deltas, but it caps how finely these rows discriminate.

## What I checked

- **Reproduced the run.** `cyrius bench benches/core.bcyr` gives 19 rows matching the report within noise (`satisfies_10dev_cuda` 367 vs 374, `to_json_8dev` 32.052µs vs 32.379µs, etc.). `cyrius fmt --check` exit 0, `cyrius lint` 0 warnings — both confirmed independently.
- **Nothing measures nothing.** Every timed body calls a function that allocates or walks the device vec; the only sub-10ns row (`empty_req`, 8ns) is the deliberate control, and 8 ns ≈ 30 cycles is the honest cost of the call + 6 loads + `vec_len` + 3 branches down to the early return at `src/core/resource.cyr:472`.
- **Setup is outside every timed region.** All 9 build fixtures before `bench_batch_start`; both `from_json` rows build their fixture text in setup.
- **Oracle parity, id by id.** All 6 `rust-old/benches/resource.rs` ids and all 7 `serde_types.rs` ids are accounted for (9 new + 4 pre-existing = 13); no gap from `core.analysis.md` was dropped and every new `bench_new` name matches the plan verbatim. Fixtures match the oracle: `large_inventory` is 8 CUDA/1 TPU/1 ROCm on 128 cores + 1 TiB; the serde fixture is the separate 8-homogeneous-CUDA one; `from_hwaccel` is 1 CPU @512 GiB + 8 CUDA @80 GiB, which divides to 81920 MB/device exactly as upstream.
- **`satisfies_10dev` really walks the full ladder, no early-out.** cpu gate reached and passed (128 ≥ 64), `required_family` unset so that arm is skipped, all 10 devices filtered into 8 hits, count gate 8 ≥ 4, memory scan hits. Branch order in `src/core/resource.cyr:448-483` matches `rust-old/src/core/resource.rs:279-333`.
- **The two divergence traps the plan flagged were both honored.** Cost is written `5000000` micro-USD, not `5`; no slot is `AGNOSAI_NO_LIMIT`. I chased the float claim further than the comment does: `_d_prettify` (lib/bayan.cyr:2505-2515) takes the `len <= kk` branch for digits "5"/k=0 and emits `5.0` with the point, and the parser sets `is_float` on byte 46 (lib/bayan.cyr:3543), so `agnosai_resource_budget_from_value` genuinely takes the float arm at :569 and not the integer fallback at :571. The comment's claim holds.
- **Symbols and arity.** All resolve as called — `profile_cpu/1`, `profile_cuda/2`, `registry_from_profiles/1` exist at lib/ai-hwaccel.cyr:890/894/3911, `reg_add_profile` does not dedupe so the registry really holds 9 profiles, and `str_data`/`str_len`/`bayan_json_v_parse_buf` match the form `src/core/task.cyr:573` uses.
- **Every cited line number in the new comments is correct** — I spot-checked all 14 (vec.cyr:30, alloc.cyr:216, bench.cyr:223, bayan.cyr:4053/3828, ai-hwaccel.cyr:3911, resource.cyr:53/140/472/543/569/571/750/819, json.cyr:23, task.cyr:565/572/573, agent.cyr:385/392). The claim that resource.cyr is the only core module without a `to_json`/`from_json` pair is true: task, agent, crew and message all have one, resource has neither.
- **Numbers cohere.** 2-dev `satisfies` 161ns → 10-dev 367ns; `devices_of_type` 257ns → `total_memory_mb` 327ns (70ns for 8 sum steps); per-field JSON cost is flat across shapes (inventory 32.05µs/43 fields ≈ 0.75µs, task 11.96µs/~10 ≈ 1.2µs, budget 3.99µs/4 ≈ 1.0µs). The one outlier is real signal, not error: `resource_budget_to_json`/`from_json` is 2.2× while every other serialize/deserialize pair is ~1.1×. That is the Grisu2 render, which is precisely what the oracle carries this id to expose — and it is genuine work, since `_d_init_tables` is guarded at lib/bayan.cyr:2201 and does not re-run per call.
- **No pre-existing bench was touched.** The diff is the header correction, two fixtures, 9 new fns, and 9 lines in `main()`; the 10 rows already in `bench-history.csv` keep their counts and their history line.

==============================================================================
FILE: server.bcyr  (10 added)
==============================================================================
Read `benches/server.bcyr` in full, the analysis plan, `rust-old/benches/server.rs` + `prompt_guard.rs`, the router/handler/serialization path, `orchestrator.cyr`, `prompt_guard.cyr`, `prometheus.cyr`, and `bench-history.csv`. Not sound. Findings, most severe first.

---

## 1. `/health` and `/ready` do not measure a response, and the three-row "probe floor" group is internally incomparable

`benches/server.bcyr:403-418`

`agnosai_route_json_a` (`src/server/routes/mod.cyr:97-108`) only stores the `bayan_json_v` pointer in a 24-byte struct. The JSON is actually rendered by `bayan_json_v_build_a` in `_agnosai_serve_send` (`src/server/serve.cyr:433-434`) — **outside** `agnosai_route_dispatch`. So `route_health_global` (460ns) and `route_ready_global` (655ns) stop at a value tree.

`/metrics` is the exception: `agnosai_route_metrics` (`src/server/routes/health.cyr:55-57`) calls `agnosai_metrics_gather`, which returns a finished `Str`, and `agnosai_route_text` stores that. So `route_metrics_global` (7.811us) **does** include its full body render.

Two consequences:
- The comment calls all three "the floor: body-limit check, resolve, handler, response". For two of the three there is no response. The 17x gap between metrics and health is a measurement-class difference, not a handler-cost difference, and nothing in the file says so.
- Against the oracle this is an early-out. `rust-old/benches/server.rs:43-62` drives `app.oneshot(...)` through the whole axum stack; `Json`'s `IntoResponse` serializes. For a handler this trivial, serialization is most of the cost, so `route_health_global` can never show a regression in the part that dominates.

**Change**: either extend `_b_dispatch_global` to call `bayan_json_v_build(agnosai_route_body(r))` when `agnosai_route_is_text(r) == 0` (and re-baseline every JSON row in the file), or drop the word "response" from :406 and state plainly that JSON rows exclude serialization while `/metrics` includes it, so the trio is not subtractable.

## 2. The `POST /crews` rationale rests on a premise the code contradicts — the crew registry *does* prune, at 1000

`benches/server.bcyr:501-515`

The comment's reason #1 cites `orchestrator.cyr:326` and states the registry "never prunes". `_agnosai_orch_register` calls `_agnosai_orch_evict_locked` at `src/orchestrator/orchestrator.cyr:238`, one line before the `map_set` it cites:

```
fn _agnosai_orch_evict_locked(o): i64 {
    var crews = load64(o + AGN_OR_CREWS);
    if (map_size(crews) < AGNOSAI_MAX_RETAINED_CREWS) { return 0; }
```

`AGNOSAI_MAX_RETAINED_CREWS = 1000` (`orchestrator.cyr:36`), and it deletes every crew for which `_agnosai_orch_is_finished` is true. Every crew in this bench is `COMPLETED` (blocking run, `_agnosai_orch_finish` stores the terminal state before the next register). So the registry sawtooths 1 -> 1000 -> 0, and at n=2000 each arm crosses the cliff **twice** — a `map_keys` over 1000 entries plus 1000 `map_delete`s folded into a single batch average that the harness reports as `min=max=avg`.

The timing distortion is small (~0.1%), but the *guidance* is wrong in a way that will be acted on: "Do not raise n past ~4,900 without minting a fresh state" is derived from the audit ring, while the thing that actually changes shape at this n is the crew-registry sawtooth. Reason #1's magnitude claim ("re-time them against 8 + n") is also wrong — it caps at 1000 — though the s2/s3 mitigation is still correct.

Related, same block: `_b_assert_post_ok(s2, ...)` at :526 runs a crew on `s2` before its batch, so `s2` enters at 10 registered crews and `s3` at 9. Immaterial in size, but it defeats the one invariant the s2/s3 split exists to hold (identical starting states).

**Change**: rewrite reason #1 and the n ceiling against `orchestrator.cyr:36/178-191`. If the row is meant to be steady-state, either drop n below the cliff (n ≤ ~950 from a fresh state) or say explicitly that the average blends pre-cliff, cliff and post-cliff regimes. Move the assert to a third throwaway state, or run it against both `s2` and `s3`.

## 3. Half the new POST rows have no guard, and their failure mode is the cheap direction

`benches/server.bcyr:526-528`, `:474-476`, `:482-483`

The plan called the status guard MANDATORY. `_b_assert_post_ok` is applied to `s2` only. `route_create_crew_arena` runs `agnosai_route_create_crew_a` on `s3` unguarded — a distinct code path (arena parse tree, `crews.cyr:525`), which is exactly the divergence a guard is for. `_b_assert_post_ok`'s own docstring at :233-239 argues this case and then the call site does not apply it.

`route_mcp_initialize_*` has no guard at all. The comment at :474-476 says "No status guard is possible here" — true of `agnosai_route_status`, false of the response. The fallthrough at `mcp.cyr:565` builds `_agnosai_mcp_error(..., "Method not found")`, which is *smaller* than the six objects `_agnosai_mcp_initialize` builds (`mcp.cyr:151-183`), and `_agnosai_mcp_unknown` logs at a level `SK_FATAL` suppresses. A typo in the hand-built 136-byte envelope therefore produces a silently faster row that reads as a win.

**Change**: guard both create_crew arms. For the mcp rows, assert on the body rather than the status — `bayan_json_v_obj_get(agnosai_route_body(r), "result") != 0`, or check `serverInfo` is present — and correct the "no guard is possible" claim.

## 4. The sizing rationale cites stale numbers, making a correct result look 25% slow

`benches/server.bcyr:298-300`

The comment interpolates from "8.0 µs at 67 B, ~273 µs at 2,880 B — both quoted in `prompt_guard.cyr:83-84`" and predicts "near 50 µs". Those are module doc-comment figures. `bench-history.csv`'s most recent run (`2026-08-11T02:51:53Z`) records `prompt_scan_clean_67b` = 9053 ns and `prompt_scan_clean_4k` = 351854 ns. The correct interpolation at 500 B is ~61.8 µs, and the measured 62.649 µs sits within 1.4% of it — the row is *exactly* on the line. As written, the comment sets up the next reader to read a healthy number as a 25% regression.

**Change**: cite `bench-history.csv`, not `prompt_guard.cyr:83-84`. (The n=10000 choice is still right: 50000 x 62.6 µs is 3.1 s, not the 2.5 s claimed.)

## 5. `route_resolve` is not the residual for the three new rows, but the comment invites the subtraction

`benches/server.bcyr:29-31`, `:403-406`

`route_resolve` times `/api/v1/dashboard/crews` (`:374`), which enters the `/api/v1` branch at `router.cyr:293` and walks the crews / a2a / … ladder. `/health` misses the prefix entirely and is the **first** literal probe (`router.cyr:402`). Its resolve is a small fraction of 314ns. So `route_health_global − route_resolve` is not "the handler", and the header's "it is in **both** numbers" no longer holds for `/health`, `/ready`, `/metrics`.

**Change**: say the probe rows carry their own (much cheaper) resolve, or add a second `route_resolve_probe` row on `/health` to give them a residual.

## 6. The fixture registers one tool where the oracle registers two, and the mcp_initialize row's headline finding depends on it

`benches/server.bcyr:120` vs `rust-old/benches/server.rs:28-29` (`EchoTool` + `JsonTransformTool`).

The analysis flagged this. Not fixing it is defensible; what is not is the comment at :467-472, which explains the initialize-slower-than-tools/list inversion by "`_b_state` registers exactly ONE tool" and adds "Register a second tool and the order flips". That publishes as a finding something the row's author knows is an artifact of a fixture that knowingly diverges from the oracle. `agnosai_json_transform_tool()` exists at `src/tools/builtin/basic.cyr:96` and is already in the include graph.

**Change**: register the second tool and re-baseline `route_tools_*` / `route_mcp_*` with a CHANGELOG discontinuity note, or drop the "order flips" sentence and state the divergence as a known fidelity gap instead of an explanation.

## 7. Comment defects that will not change a number but will mislead

- **Self-contradiction on placement.** Header :41-49 argues a delta whose halves land in different report files "is unreadable" — and :294-300 then relies on exactly that cross-file delta (against `orch.bcyr`'s 67 B and 2,880 B rows) to justify the 500 B length and the iteration count.
- **:318** quotes "62.1 µs clean vs 2.06 µs here" and "30.2x". The committed run is 62.649 / 2.086 = 30.03x. The comment reports a different run's numbers as the row's finding.
- **:462/:470** call the tools/list envelope 46 bytes and the initialize envelope 135; they are 45 and 136.

---

## Checked and clean

- No hoisted/empty loop bodies: every timed body calls a function whose result is discarded but which allocates; all reported times are ≥460ns, none near the 0-2ns red flag.
- Setup is outside every timed region: `_b_clean_500`, the suspicious corpus, `field`, `_b_crew_body`, `_b_mcp_init_body`, `s2`, `s3` are all built before `bench_batch_start`.
- Symbols and arity all resolve: `str_sub(s,start,len)` (`lib/str.cyr:201`, so `0,500` is 500 bytes — correct), `agnosai_prompt_sanitize(text,field)`, `agnosai_prompt_scan_input(text)`, `agnosai_route_status(r)` (`routes/mod.cyr:126`), `HTTP_OK` (`lib/http.cyr:32`), `AGNOSAI_SCAN_CLEAN`.
- The 31-pattern count is right (`grep -c _agnosai_prompt_contains_ci` = 32 including the definition), and `"ignore previous instructions"` genuinely is table entry #1 (`prompt_guard.cyr:107`), so the early-exit guard at :340 is load-bearing and correct. Corpus is 460 + 29 = 489 bytes as documented. `_b_clean_500`'s 45-byte repeat unit is right and the analysis's "44" was wrong.
- `/metrics` counter claim `8/0/8/0/0/0.000000` verified against `prometheus.cyr:200-235` and `crew_runner.cyr:1210/1254-1255` — the cited lines are accurate, and the placement before the POST /crews rows is the right call.
- `route_metrics_global` 7.811 µs vs `metrics_gather` 7457 ns in history: a 354 ns delta against `route_resolve`'s 314 ns. Coherent.
- No dangling-arena hazard in the create_crew arena arm: `agnosai_crew_req_from_value_a` is handed `default_alloc()` and uses `str_clone_a` / `vec_push_a` throughout (`crews.cyr:221-260`), so nothing retained points into the reset arena.
- Inserting three rows ahead of the existing ones did **not** shift them: every pre-existing row is within ±3% of `2026-08-11T02:51:53Z` (dashboard_agents 9628→9626, mcp_global 3986→3959, tools_global 2305→2243).
- All 8 gaps from `server.analysis.md` are present; nothing was silently dropped. The two deliberate deviations (prompt rows in `server.bcyr` not `orch.bcyr`; n=10000 not 50000) are both defensible on the merits, and only their stated justifications are wrong (findings 4 and 7).

==============================================================================
FILE: tools.bcyr  (8 added)
==============================================================================
`benches/tools.bcyr` compiles and all 8 ids run, but several things are wrong. Ranked:

## 1. `tool_registry_register` does not reproduce, and the header states point numbers and derives ratios from a single un-replicated sample — `benches/tools.bcyr:358-361`, `:365-385`

Three runs of the same unmodified arm:

| source | value |
|---|---|
| header comment (`:358`) | 610ns |
| the authoring agent's reported run | 648ns |
| my re-run just now | **766ns** |

That is a +26% spread. Every other new arm reproduces within 6% (`get_5`: 103/103/109). So `tool_registry_register` as written cannot detect any regression smaller than ~30%, which is most of them.

Cause: it is the only arm whose *timed body* faults in fresh pages. `map_set` → `_map_grow` allocates `new_cap*24` on the no-free global bump for every doubling from 16 to 262144 — Σ ≈ **12.6 MB**, each freshly `memset`, plus ~2.4 MB of `str_clone` buffers (`lib/str.cyr:208`, `src/tools/registry.cyr:46`). All first-touch inside `bench_batch_start`/`stop`. The grow work is legitimately part of amortised insert; the page-fault cost is not, and it is what moves run to run.

Made worse by the harness: `bench_batch_stop` (`lib/bench.cyr:220`) is called exactly once, so `min == max == avg` and the reported `(min=… max=…)` conveys **zero** variance information. The one arm with 26% real variance is the one that displays as a clean point estimate.

Also factually wrong in the same comment: *"against 112ns for a `get` on the same table"* — there is no `get` arm at 100k entries (the largest is 500), and `112` is not a number from the reported run either. And `:349` says "~14 doublings"; from cap 16 to 262144 it is 14 grows, fine, but `:361`'s "61ms total" is derived from the stale 610ns.

**Change:** wrap `bench_batch_start`/`bench_batch_stop` in an outer repeat loop (as the arm's own `min`/`max` fields are designed for) so the spread is visible, or split the fixture into several smaller registries. At minimum delete the "610ns / 5.4x / 61ms" derivation and the false "get on the same table" comparison.

## 2. The 5/50/500 sweep conflates registry size with key length, and its stated conclusion is contradicted by the file's own neighbouring arm — `benches/tools.bcyr:265-268`, `:232-241`

`:265-268` concludes *"A 100x registry costs 11% more, and all of that is cache."* Neither half survives:

- **The file's own `tool_registry_get` (3 tools, `:204-215`) is the slowest of the four**: 122ns reported / 130ns on my run, against `tool_registry_get_500` at 114/120ns. A 3-entry map losing to a 500-entry map means registry size is not what these arms are resolving.
- The real variable is key length. `hash_str_v` (`lib/hashmap.cyr:84`) is a per-byte FNV loop. The 3-tool arm hashes `"json_transform"` = 14 bytes; the new arms hash `"tool_N"` = 6 bytes at n=5, but 6–8 bytes averaging ~7.4 at n=500. So a real share of the 103→114 slope is 1.4 extra hashed bytes, not cache residency. "All of that is cache" is asserted, not shown.

**Change:** make `_b_tool_names` (`:232`) emit fixed-width keys (`tool_000`…`tool_499`) so the three points differ *only* in registry size, and either rewrite the conclusion or state the confound.

## 3. `:38-39` claims the new sweep is comparable to the existing arm; it is not — `benches/tools.bcyr:38-39`, `:274-276`

> *"The existing `tool_registry_get` already made that call, so the sweep stays internally comparable."*

Prebuilding the key is only one of the differences. The new timed body is

```
agnosai_tool_registry_get(reg, vec_get(names, i % n_tools));
```

`n_tools` is a runtime parameter, so `%` is a genuine 64-bit division that cannot be strength-reduced, plus a `vec_get`. The old arm at `:211` has neither. The measured inversion in finding 2 is the proof that these two shapes are not on the same scale.

**Change:** drop the claim, or replace `i % n_tools` with a wrapping counter (`k = k + 1; if (k == n_tools) { k = 0; }`) so the divide leaves the window.

## 4. Nothing verifies the `has` "hit" arm actually hits, and the null result is elevated to a finding — `benches/tools.bcyr:297-306`

`:297-302` reports 107ns hit / 109ns miss (108/109 in the agent's run, **116/116** in mine) and concludes *"indistinguishable, and that is the finding."*

There is no evidence in the benchmark that the hit arm hits. `agnosai_tool_registry_has`'s return is discarded; the map keys are `str_clone`s compared by content via `str_eq`, so today `"tool_25"` does resolve — but if that ever stopped being true (key-type change, naming change, a `_b_registry_of` bug) both arms would silently become misses and still report ~110ns. The arm would look healthy while measuring one thing twice.

Compounding it, the two arms differ in **two** variables, not one: `"tool_25"` is 7 bytes, `"nonexistent"` is 11. The miss arm hashes 4 extra bytes — in exactly the arm the hypothesis predicts should be slower. A "no difference" result from a design that has a confound pushing the same direction as the hypothesis is not a finding.

**Change:** assert the fixture outside the timed region before `bench_new` (e.g. bail if `agnosai_tool_registry_has(reg, hit) != 1`), do the same for the `get` family (`agnosai_tool_registry_get` returning 0), and use a same-length absent key such as `"tool_99"` so hit/miss differ only in presence.

## 5. None of the 8 new ids exist in `bench-history.csv`, and there is no CHANGELOG entry

`grep` over `bench-history.csv` returns only `tool_registry_get`; zero rows for any of the 8. `scripts/bench-history.sh` auto-discovers, so it just was never run (CLAUDE.md work loop steps 4 and 8). The consequence is that the header's "Measured …" numbers are the de-facto baseline yet live only in a comment where nothing can falsify them — which is how finding 1's 610ns survived two contradicting runs. `git diff CHANGELOG.md` also has no entry for the seven new arms.

**Change:** run `./scripts/bench-history.sh`, then reconcile the header numbers against the committed rows; add the CHANGELOG entry.

## 6. `:313` — "four `str_from_a` copies each" is wrong

`str_from_a` (`lib/str.cyr:24`) allocates a 16-byte `Str` header and **borrows** the literal; there is no byte copy. The per-tool arena cost is 24 (schema) + 152 (params vec: 24 header + 16×8 backing, `lib/vec.cyr:30`) + 32 (param) + 64 (four `Str` headers) ≈ 272 B — the params vec, not the strings, is the dominant term. The downstream "~130 KB per call" at `:43` happens to come out right (500×272 + two ~8 KB vec-growth chains ≈ 152 KB), but by accident.

## 7. `:42` cites the wrong function in `benches/server.bcyr`

The comment cites `benches/server.bcyr:164` as the arena/`reset_via` idiom. Line 164 is inside `_b_dispatch_global` — the arm that deliberately does *not* use an arena. The idiom being referenced is `_b_dispatch_arena` at `benches/server.bcyr:180-196`.

---

**Checked and sound** (so these are not on the list): symbols and arity all resolve as called — `agnosai_tool_new/5` (`src/tools/native.cyr:342`), `_agnosai_echo_execute/2` (`src/tools/builtin/basic.cyr:33`), `agnosai_tool_schema_new_a/3`, `agnosai_tool_schema_param_a/6`, `agnosai_tool_registry_list_a(a, r)` arg order. `_b_named_schema` is a faithful Cyrius `NamedEcho` — it mirrors `_agnosai_echo_schema` (1 param, same strings) with the name taken from `ctx`, matching `tools.rs:36-41`. Setup is outside the timed region in all four helpers, including the `list` warm-up call. The 4 MB arena is comfortable against a real ~152 KB high-water at 500 tools, and `arena_reset` (`lib/alloc.cyr:488`) is O(1) with no memset, so the oversize does not distort the `list` arms. `list_50`/`list_500` divide at 9.90x for 10x work — coherent. The `sakshi_set_level(SK_FATAL)` justification at `:439-444` is **correct** and the analysis plan it overrides was wrong: default `_sk_log_level = SK_INFO (3)`, `_agnosai_registry_log` guards `3 < SK_DEBUG(4)` and already early-returned, and load_testing's only ≤INFO logger `_agnosai_lt_log` is reached solely from `_agnosai_load_testing_execute` (`src/tools/builtin/load_testing.cyr:561,572`), which no arm calls — so no existing id was silently rebaselined. All 7 planned tools gaps are present (get 50/500, has hit/miss, list 50/500, register); nothing was dropped, and the extra `tool_registry_get_5` with 5 real stubs is closer to `tools.rs:58-69` than the plan's mapping onto the 3-builtin arm.

==============================================================================
FILE: fleet.bcyr  (14 added)
==============================================================================
## Findings

### 1. SEVERE — Four of the six placement shapes never enter the sort, and the two the header names as the sort/tiebreak stress cases are exactly the two that short-circuit

`agnosai_sort` → `vec_sort_by` (`/home/macro/Repos/agnosai/lib/vec.cyr:340-352`) runs an O(n) "already ordered" pre-check and **returns before `_vec_introsort`** if `cmp(prev, cur) > 0` never fires. `src/order.cyr:92-93` says so in the comment on the very line the bench header cites (`src/order.cyr:94`).

- `/home/macro/Repos/agnosai/benches/fleet.bcyr:564` `fleet_rank_nodes_gpu_affinity_200` — all 133 survivors score 0.08192 and their `AGN_PR_INDEX` is ascending, so `_agnosai_place_cmp` returns −1 for every adjacent pair → ordered → **no sort runs**. The ⚠ at lines 556-563 claims the opposite ("puts the whole sort past the introsort's 16-element insertion-sort threshold").
- Worse, the same block claims this bench guards "the case `src/fleet/placement.cyr:20-23` says a score-only comparator diverges on." With a score-only comparator every adjacent compare returns `0`, `c > 0` is still false, still ordered, still no sort → **identical timing**. The benchmark is structurally blind to the regression it claims to guard.
- `benches/fleet.bcyr:463` `fleet_place_gpu_affinity_50` — same all-tie/ascending-index case; comment at 457-462 says "and a full sort".
- `benches/fleet.bcyr:479` `fleet_place_balanced_50` — 50 strictly-descending scores → ordered → no sort.
- `benches/fleet.bcyr:540` `fleet_place_hw_req_50` — Balanced policy, survivors keep ascending pre-filter indices → strictly descending → no sort; comment at 537-539 says "adds the 50-node score + filter + sort around it".

Only `fleet_place_locality_50_3caps` (scores cycle 1/3, 2/3, 1, 1) and `fleet_rank_nodes_cost_200` (CPU nodes at 1.0 interleaved with tiny GPU scores) actually reach `_vec_introsort`.

**This is also the answer to the number-coherence check.** locality 12.405us vs balanced 3.427us at identical 50-node/50-result size, and cost_200 30.171us vs gpu_affinity_200 11.930us, are not "heaviest tiebreak load" — they are sort-runs vs sort-skipped. The header's causal story is inverted in both places.

Change: add a shape whose ties are *not* already in index order (e.g. rank a reversed fleet, or interleave the GPU nodes so survivor indices are not monotonic) so `_vec_introsort` + the index tiebreak are actually priced; and rewrite the four comments to state that the pre-sorted fast path is taken.

### 2. SEVERE — The header's headline claim, `clock_epoch_secs()` = 1.318us, is unsourced and contradicted twice by the repo's own numbers

`/home/macro/Repos/agnosai/benches/fleet.bcyr:72-86`.

- No benchmark of `clock_epoch_secs` exists anywhere in `benches/`, and `bench-history.csv` has no clock row. "It measured **1.318us** on the same host as everything below" is not reproducible from the tree — against CLAUDE.md's "the CSV history is the proof".
- `/home/macro/Repos/agnosai/lib/bench.cyr:6` documents the *identical* mechanism — a raw `syscall(228, …, &ts)`, no vDSO, same as `lib/chrono.cyr:75` — at **~120ns per call**. That is 11x lower.
- `uuid_v4_generate` = **501ns** in `bench-history.csv` (2026-08-11), and `agnosai_uuid_v4` (`src/id.cyr:41-44`) makes a real `sys_getrandom` kernel entry (`lib/random.cyr:34-42`) plus an `alloc` plus the version stamp. A `clock_gettime` costing 2.6x an entire getrandom-plus-work call is not credible.

Consequence: "1 clock = 63% of the call" for reach_barrier (line 79) is more likely ~6%, and lines 83-86 direct a stdlib clock-source issue on that basis. The likelier dominant term is `_agnosai_csm_arrived` (`src/fleet/state.cyr:369-382`), which on the first call of each cycle does `map_new_str()` + `vec_new()` + two `map_set`s inside a 300-entry outer map.

Change: add a `clock_epoch_secs` baseline bench to this file, the way `benches/core.bcyr:105-126` gives uuid its subtractable baselines, and re-derive — or delete lines 72-86 and the two decompositions at 78-79 until there is a measured number.

### 3. MEDIUM — Six of the plan's 19 gaps shipped nowhere

`fleet.analysis.md` lines 148-213 route the six `scoring.rs` gaps to `benches/orch.bcyr`, and `benches/fleet.bcyr:11-13` restates that routing — but nothing executed it. `grep score_agent benches/` returns zero; `orch.bcyr`'s only ranking bench is `rank_agents_16` (`benches/orch.bcyr:255`), which the plan itself files under "Already covered" with the caveat that its 16 agents are identical and the insertion sort does zero shifts. Missing: `score_agent_rich_context`, `score_agent_no_context`, `score_agent_gpu_required`, `score_agent_domain_mismatch`, `rank_agents_100_varied`, `rank_agents_1000_varied` — the last of which the plan calls "THE ONE THAT MATTERS" (the port replaced Rust's merge sort with a hand-rolled insertion sort at `src/orchestrator/scoring.cyr:294-303`, potentially O(n²) on an attacker-influenceable agent count).

### 4. LOW — Stale first-run numbers hardcoded in comments

`benches/fleet.bcyr:78-79` quote 4.058us / 2.077us; the shipped run reports 3.981us / 2.045us. `benches/fleet.bcyr:366-367` quote "548ns for allocate against 149ns for release"; the run reports 512ns / 145ns. They are labelled "first run", but a comment carrying a number the file's own output contradicts reads as an error on every future run.

### 5. LOW — Citation off-by-one

`benches/fleet.bcyr:74` cites `lib/chrono.cyr:76-78` for the raw Linux `syscall(228, CLOCK_REALTIME, ..)`. It is at `lib/chrono.cyr:75-76`; 77-78 are `#endif`s.

---

## What I checked and found clean

- **Setup outside the timed region** — verified for all 14: `bench_batch_start` comes after the fixture in every case (fleet.bcyr:134, 169, 231, 273, 317, 377, 346, 399, 440, 450). `_b_compute_release_only` correctly refills untimed at 374-376 before starting at 377.
- **Symbols and arity** — all 28 called `agnosai_*` symbols exist with matching arity (`state.cyr:270/304/387`, `coordinator.cyr:119/165/227`, `gpu.cyr:153/166/307/322/344`, `placement.cyr:78/91/97/103/115/271/292`, `registry.cyr:83/102/108`, `resource.cyr:169/266/273/279/285/391`, `id.cyr:183`).
- **`AGNOSAI_ACCEL_ANY` (−1) at all three allocate sites** (319, 342, 350) — the reject-path trap is avoided.
- **Nothing measures nothing** — every timed body mutates or allocates; lowest reported figure is 71ns; no candidate for hoisting.
- **Retention-cap arithmetic** — 900 < 1000 runs, 500 runs / 5000 tasks, 800 runs / 8000 tasks all stay under `AGNOSAI_MAX_RETAINED_RUNS` (`state.cyr:53`) and `AGNOSAI_MAX_RETAINED_TASKS` (`coordinator.cyr:71`); no O(n²) eviction scan fires.
- **Fixture arithmetic against the oracle** — `make_fleet` 17 CPU / 33 GPU at n=50 and 67/133 at n=200; `vram_per_gpu` not multiplied by `num_gpus`; the hw_req comment's correction of the plan (33 nodes allocate `type_match`, not 50, because CPU nodes hit the `min_cpu_cores` gate at `resource.cyr:449-450` first) is right, and so is "25 nodes rank".
- **`_b_compute_allocate` fills exactly 192 with no reject**, and `_b_compute_release_realloc`'s 8 holders do land on 8 distinct devices via last-wins (`gpu.cyr:283`) with a map-stable loop.
- **Barrier names vary within a run** and node ids compare by content (`_agnosai_dcs_set_has`, `state.cyr:104-109`, uses `str_eq`), so the AllReached early-out at `state.cyr:399-405` is not silently short-circuiting.
- **Iteration counts** match the plan and the reported output exactly (45000/90000/20000/500000/38400/38400/200000/500000/20000×3/5000×3), and min = max = avg only on the single-batch shapes, as the header states.

==============================================================================
FILE: orch.bcyr  (37 added)
==============================================================================
**7 problems. The benchmarks themselves are structurally sound — every timed region I checked is correctly scoped and no row measures nothing. The defects are memory, missing failure guards, and two comment claims that do not survive measurement.**

---

**1. [severe] Peak RSS triples: 1.16 GB → 3.44 GB. The header's central memory claim is false as written.**
`benches/orch.bcyr:38` — *"⚠ Iteration counts here are MEMORY-bound, not time-bound"* — and it then justifies only two of the 37 rows (`ipc_frame_1mib_transport` at 50, `orchestrator_new` at 20000). Measured on this machine by polling the `bench` process (5 runs, all rows verified passing):

| variant | peak RSS |
|---|---|
| `git show HEAD:benches/orch.bcyr` (35 pre-existing rows) | **1,161,988 KB** |
| current file (72 rows) | **3,436,652 KB** |
| current minus the 3 `_b_load_dag` calls | 2,617,628 KB → **the load_dag block alone is ~820 MB** |
| current minus `scheduler_load_dag_linear_50` only | 3,150,348 KB → **that one row is ~285 MB** |
| current minus the 4 `_b_pattern` calls | 3,160,496 KB → **~276 MB for the four** |

The per-call estimates the counts were chosen from are 2–3x low. `benches/orch.bcyr:1760` says linear-500 *"allocates ~220 KB"* per call; the block measurement puts linear-500 + wide-100 at ~535 MB, so linear-500 is nearer ~500 KB/call. `scheduler_load_dag_linear_50` measures ~58 KB/call against the plan's ~22 KB — and it was left at n=5000 while its 500-task sibling was cut to n=500 for exactly this reason, so the row that got the memory discipline ends up cheaper than the one that didn't. `benches/orch.bcyr:1729` justifies the pattern counts purely on timing resolution (*"the per-op cost is hundreds of nanoseconds, so the resolution is ample"*) and never prices their allocation at all — they are ~69 MB each.

The source plan warned about precisely this (`orch-queueing.analysis.md:514`: *"raising any of them 10x will push RSS into the gigabytes"*). It is in the gigabytes. `.github/workflows/ci.yml:120` runs bare `cyrius bench` on `ubuntu-latest`, so it survives at 16 GB — but a 3x jump is not what the header claims was done, and nothing in the file records it.

**Change:** cut `scheduler_load_dag_linear_50` to n≈1000 and the four `pattern_match_*` rows to n≈50000/25000 (per-op costs are 458–1469 ns; resolution is still ample at a tenth of the iterations), then correct `:38` and `:1760` with the measured per-call figures rather than the plan's estimates.

**2. `_b_ipc_bind_close` never checks the bind — a failed bind reports a *faster* number, silently.**
`benches/orch.bcyr:1321` `var fd = agnosai_ipc_bind(path);` followed by `sys_close(fd)`. On failure `agnosai_ipc_bind` returns `AGNOSAI_IPC_ERR` (`-2`, `src/orchestrator/ipc.cyr:50,214`) and `sys_close(-2)` returns EBADF cheaply, so the whole timed body degenerates into "unlink attempt + failed socket setup" and posts a plausible, smaller number to `bench-history.csv`. The path is the fixed `/tmp/agnosai-bench-ipc.sock` (`:1216`), so a stale file owned by another user makes this the permanent state. The author already applies the guard one function down at `:1344` (`if (srv == AGNOSAI_IPC_ERR) { return 0; }`), so this is an inconsistency, not an oversight of the hazard.

**Change:** guard the loop body, and fail loudly rather than returning 0 — see #4.

**3. The two approval rows drop the oracle's assertions, and they are the assertions that keep the rows off the reject path.**
`rust-old/benches/approval.rs:17` is `assert!(rx.is_some())` and `:32` is `assert!(delivered)`. The port ignores all three return values: `benches/orch.bcyr:1176-1177` (`agnosai_approval_request` / `agnosai_approval_cancel`) and `:1199-1200` (`request` / `submit_decision`). If the gate ever reaches `AGNOSAI_APPROVAL_MAX_PENDING` (`src/orchestrator/approval.cyr:229`), `request` returns 0, `cancel`/`submit_decision` then take their `ch == 0` early return, and both rows quietly become "map miss + suppressed log" — reporting a large improvement. `approval_request_cancel_512_pending` sits at 513 of a 1000 cap, so the margin is under 2x. This is the same failure mode the author explicitly defends against 350 lines later in `_b_relay_receive_dupes` (`:1541-1542`, with the reasoning at `:1513`), using exactly the right technique.

**Change:** after each loop, assert the terminal state and `syscall(60, 1)` on mismatch — e.g. `agnosai_approval_gate_pending_count(g) != pending`.

**4. The four IPC rows silently drop themselves from the CSV on setup failure.**
`benches/orch.bcyr:1254`, `:1289`, `:1379` (`if (sys_socketpair(...) != 0) { return 0; }`) and `:1344` (bind failure) all return 0 without calling `bench_report`. `main` discards every `_b_*` return, so the run still prints `=== 1 passed, 0 failed ===` with a missing row — and a missing row in `bench-history.csv` reads as "not run", which is indistinguishable from "no regression". The file states this exact principle at `:1513-1515` and then does not apply it here.

**Change:** `syscall(60, 1)` on these paths, matching `:1542`.

**5. `_b_ready_tasks` ignores `agnosai_scheduler_load_dag`'s return value.**
`benches/orch.bcyr:1013`. A 0 return (cycle detected, `src/orchestrator/scheduler.cyr:253`) leaves `AGN_SCH_DAG_KEYS` at the empty vec `agnosai_scheduler_new` installed, and `agnosai_scheduler_ready_tasks` then walks nothing and returns in ~50 ns. Both `scheduler_ready_tasks_wide_100` and `..._half_done` would post a 100x "improvement" that nothing in the run would flag. Cheap to guard; the DAG shape is fixed so this is a fixture-rot tripwire, not a live risk.

**6. `orchestrator_new` hoists setup the oracle times — an undocumented divergence in a file that documents every other one.**
`benches/orch.bcyr:1639` `var budget = agnosai_resource_budget_default();` sits outside `bench_batch_start`. The oracle's timed closure is `Orchestrator::new(ResourceBudget::default()).await` (`rust-old/benches/orchestrator.rs:66`) — the `default()` construction is inside. `agnosai_resource_budget_default` calls `agnosai_resource_budget_new` (`src/core/resource.cyr:499-501`), a real allocation. Small in absolute terms against 2.86 µs, but every other hoist in the new block (the pubsub payload at `:1069-1071`, the pattern strings at `:1034-1035`) carries an explicit divergence note and this one carries none.

**7. The pubsub-ladder comment models a term as fixed that varies across the three rows.**
`benches/orch.bcyr:1057-1060` attributes *"a ~3.5 µs fixed cost (the TopicMessage, the `clock_now_ns`, the `map_keys` vec **and the per-pattern matches**)"*. The pattern count is not fixed — it is 1, 3, and 5 across the ladder (`src/orchestrator/pubsub.cyr:207-209` calls `agnosai_matches_pattern` once per map key). Solving my three measured points (3.084 / 12.248 / 74.349 µs) for `a·matches + b·pushes + c` gives **a ≈ 0.68 µs per `matches_pattern`, b ≈ 0.87 µs per saturated push, c ≈ 1.5 µs genuinely fixed** — the push figure is right, the fixed figure is 2.3x too large because it has absorbed a per-pattern term. Anyone using the stated model to predict a fourth ladder point gets the wrong answer.

**Also worth a decision now, not later:** the header at `:11-18` flags that the six `relay_*` rows measure `src/fleet/relay.cyr` and belong in `benches/fleet.bcyr` (which exists, covers `fleet.rs` + `placement.rs`, and has no id collision — I confirmed both). Leaving a self-identified misplacement in the tree with a note is how it becomes permanent.

---

**Checked and clean** — against your six questions:

- **Nothing measures nothing.** Every one of the 37 rows has a live timed body; the smallest (`pubsub_subscribe_at_cap`, 100 ns) genuinely runs a mutex round trip plus a 10000-entry map miss plus the cap reject (`src/orchestrator/pubsub.cyr:177-183`). No row is hoistable — every loop body has a side effect or an unelidable call.
- **No setup is inside a timed region.** I walked all 37. The three cases where work *is* inside (`agnosai_uuid_v4_str` in the approval rows, `str_cat` in `pubsub_subscribe_new_pattern`, `_b_o_spec` in the `run_crew` rows, `agnosai_task_new` in `scheduler_enqueue_100_build`) each match the oracle's own closure exactly — verified against `approval.rs:14`, `pubsub.rs:90`, `orchestrator.rs:82`, `scheduler.rs:14`.
- **No early-out substitutes for the oracle's ladder.** `_b_pubsub_ladder`'s 256-deep prefill (`:1094-1096`) does put every timed push on `agnosai_chan_push_lossy`'s evict-and-retry branch — I confirmed `chan_try_send` rejects at `count >= cap` with cap exactly 256 (`lib/thread.cyr:622-623`, `chan_new` at `:529`). `agnosai_scheduler_ready_tasks` does not mutate the scheduler, so iterations 2..n do the same work as iteration 1.
- **The numbers cohere.** `ipc_throughput_100_same_conn` = 892 µs vs 100 × 8.989 = 899 µs (0.8%). dequeue 100→1000 solves cleanly to a ~42 ns pop + ~1.96 ns/shifted-slot, matching the claimed `vec_remove(q, 0)` quadratic. The enqueue pair puts `agnosai_task_new` at 1.86 µs/task, consistent with the `getrandom` floor implied independently by `approval_request_cancel`. The deep-pattern row's *"four failed probes (i=3,4,5,6), peak depth 7 against a cap of 32"* is exactly right — I traced it.
- **Symbols and arity all resolve**, including the order-sensitive `agnosai_relay_message_new(seq, from, to, topic, payload)` against `src/fleet/relay.cyr:150`. I compiled and ran the file myself: 72 rows, `=== 1 passed, 0 failed ===`.
- **No gaps dropped.** All 16 `orch-queueing.analysis.md` gaps landed, plus all 5 of its recommended companions, plus all 13 `orch-transport.analysis.md` gaps. The eight corrections in `orch-queueing.audit.md` and the eight in `orch-transport.audit.md` are each reflected in the bench comments — including the ones that overturned the analysis (the oracle sorts nothing before `kahn_sort`; edge dedup is 9,900 not 10,100; the dep loop breaks so sink costs ~1,325; `MAX_MATCH_DEPTH` is never reached; the lossy push is three mutex round trips; the accept row's label is server-side-only). The in-file "Measured X" figures all reproduce on my runs within noise.

