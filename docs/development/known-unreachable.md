# AgnosAI — Known-unreachable code

Moved out of the roadmap at 2.3.0, as none of it is work. Kept for oracle shape, and written down so
nobody re-derives it as a finding.

Not defects and not owed — recorded so nobody re-derives them as findings. Each is
documented in place as unreachable rather than implied to fire: `agents.rs`'s
serialize skip, the cycle-detector's `== 2` memoization arm, `crews.rs`'s profile
skip, and `crew_runner.rs`'s personality prompt block (bhava; see *What agnosai is*).

The entries below came from the 2026-08-04 sandbox audit
([`m7-audit-2026-08-04.md`](../audit/m7-audit-2026-08-04.md)) and the fleet and telemetry ports. Each
was reached and correctly rated not a defect; "no test covers this" is true of each and will
stay true:

- **`kavach_bridge`'s "start failed" arm.** kavach's `valid_transition` accepts
  `CREATED -> RUNNING` unconditionally, so a freshly created sandbox always
  starts. Kept because the oracle has the arm. It is the one of the three error
  paths that *can* carry a kavach error code, and now does.
- **`fleet/gpu`'s two `saturating_sub` guards (added 2026-08-08).** `used` moves
  only by `allocate` (+) and `release` (-), and `release` removes the record
  before subtracting, so `used` is always >= any single record and the guard
  cannot fire. The oracle needs it because Rust's `u64` underflow panics in
  debug. Mutation-verified as unreachable: deleting the guard in `release`
  leaves all 65 assertions green. Kept for oracle shape, documented in place.
- **`fleet/federation`'s `elect_by_lowest_id` `None` arm (added 2026-08-08).**
  The candidate list is the online peers **plus this cluster's own id**, so it
  is never empty and `candidates.first()` is always `Some`. The port returns the
  winner and never 0; a lone cluster elects itself. Kept in the doc comment
  only, and asserted from the reachable side (`fleet_federation.tcyr` pins that
  a peerless manager still returns a winner).
- **`fleet/federation`'s strict `>` liveness comparison (added 2026-08-08).**
  `check_liveness` demotes on `elapsed > timeout` where `NodeRegistry`'s sweep
  uses `>=`. Discriminating the two needs elapsed to land *exactly* on the
  threshold, and the sweep reads its own `clock_now_ns()` after a test has set
  the instant, so the boundary is unreachable from outside. Both directions are
  asserted with a 1 ms margin; the strict comparison itself is documented in
  place, not claimed as tested. Not a defect — the difference from `registry` is
  the oracle's, and reproducing it is the point.
- **`telemetry/otlp`'s two-arena separation (added 2026-08-09).** The export
  ring keeps fragments and the batch document in different arenas so `drain`
  cannot reset the arena it is about to read from. Two mutants of this survive
  every assertion — allocating fragments into the document arena, and resetting
  the document arena after the build instead of before — because **a bump-arena
  reset only rewinds an offset**. The bytes stay intact, so use-after-reset
  reads correct data in a single-threaded test and the output is identical.
  A test pins that two arenas exist (killing the collapse-to-one
  simplification); the rest is correct by construction and by review, not by
  test. Do not "simplify" it on the evidence of a green suite.
- **`fleet/topology`'s `total_pairs == 0` guard (added 2026-08-08).** It is
  reached only when `gpu_count >= 2`, where `n * (n - 1) / 2 >= 1`, so it cannot
  fire. Kept because the oracle has it. Mutation-verified from the reachable
  side: the six other topology mutants all die, and this branch has no input
  that reaches it.
- **`fleet/federation`'s three unread config fields (added 2026-08-08).**
  `endpoint`, `seeds` and `election_timeout` are stored and never read by any
  method, exactly as upstream: the oracle logs `seeds.len()` once at startup and
  its `election_timeout` doc claims a randomization nothing performs. Accessors
  exist so a caller that owns the election timer has one place to configure it.
- **`fleet/state`'s `is_checkpointing` is observably always false.** `checkpoint()`
  sets it, pushes, and clears it before returning, and every oracle method takes
  `&mut self`, so no caller can observe it set. The field and both writes are
  ported anyway: the oracle's comment says it exists so barrier operations can be
  queued against it, a contract for a future concurrent caller rather than dead
  code (`src/fleet/state.cyr:26-32`).
- **`fleet/cost_planning`'s missing `model_throughput` `None` arm.** The oracle's
  `let Some(..) = model_throughput(..) else { return 0.0 }` is unreachable — every
  branch of `model_throughput` returns `Some`, including its fallthrough. There is no
  Cyrius branch because `_agnosai_cost_tier` is total; the oracle's `0.0` is documented
  in place rather than reproduced as a branch nothing can enter
  (`src/fleet/cost_planning.cyr:224-228`).
- **Two log-only fixes whose mutants survive — L5 and L9.** Nothing in this tree
  captures sakshi output, so `sakshi_warn`'s corrected length and the manager's
  restored dispatch `debug!` cannot be asserted. Both are correct; neither is
  verifiable. Said plainly in the audit document's Status section rather than
  counted as mutation-verified, and repeated here because a future sweep will
  otherwise flag them as untested code.
