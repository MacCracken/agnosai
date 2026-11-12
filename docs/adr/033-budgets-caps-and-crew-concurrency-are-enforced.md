# 033 — Budgets, caps and crew concurrency are enforced

**Status**: Accepted, shipped in 2.4.1
**Date**: 2026-11-12

## Context

agnosai carried every limit the Rust line had, and enforced one of them.

- **`orchestrator/budget.cyr` and `orchestrator/multi_tenant.cyr` had no callers** outside their own files. A crew's
  tokens and cost were counted onto its results after the fact and checked against nothing, and no tenant was ever
  looked up.
- **`max_concurrent_tasks` was stored and read by nothing.** The Rust orchestrator held a semaphore of it (default 10)
  around every run. The port ran every crew it was given at once, and `submit_crew` spawned a thread for each. agnostic
  removed its `AGNOSTIC_CREW_MAX_CONCURRENT_TASKS` because of this, and its roadmap gets it back on this release.
- **A parallel crew ran fixed batches.** One slow task left the batch's other slots idle until it ended, and since
  2.4.0 a task waiting on a reviewer held its whole batch for as long as the reviewer took (ADR 032).
- **The tool loop's limits were process-wide** (ADR 029), and ADR 029 left per-crew limits to this release.
- **A burst of TLS handshakes overflows sigil's lane pool.** sigil verifies a certificate on one of 64 lanes and
  refuses a verify that finds none free, which reads as a bad signature: of 100 handshakes at once, 36 fail (*Waiting
  on another project*). agnosai runs 100 pool workers and a thread per crew.

agnostic holds agnosai to one rule recorded with turnstone's (its roadmap, F5): **budgets subdivide**, from crew to
task to delegated worker. Anthropic measured multi-agent runs at about 15× the tokens of a chat (2025), and Claude
Code caps its subagents at 20 at once. The cyrius stdlib has no counting semaphore.

## Decision

### A counting semaphore, first come first served

`src/semaphore.cyr`, port-local. A waiter joins a queue and a release hands its permit straight to the oldest waiter,
so a caller that arrives while others wait queues behind them, and no waiter can be passed over for good. A waiter may
give up at a deadline or when a word it names turns nonzero, and gives up only what it did not hold: it leaves the queue
under the lock, so a permit cannot be handed to a waiter that has gone. Whoever sets that word wakes the queue
(`agnosai_semaphore_wake_waiters`), so a waiter gives up at once; it polls only once a second, for a word set without.
Its node is on its own stack, and a grant is the releaser's last touch of it, as `src/done_word.cyr` sets its word.

### Crews wait their turn

**At most `max_concurrent_tasks` crews run at once**: the orchestrator budget's, or the Rust line's 10 when it sets
none (`unwrap_or(10)`), as the timeout falls back to its hour. A crew waits, `pending`, behind those that came before
it, whether a caller runs it inline (`POST /api/v1/crews`) or submits it. Its deadline starts when it runs, as the
Rust line's timeout wrapped only the run. A cancel ends the wait at once, as `cancel_crew` wakes the queue: the crew
ends `cancelled` with no events, as ADR 032's `pending` → `cancelled` says, and its channel closes.
`agnosai_orchestrator_crews_queued` says how many wait.

### A parallel crew is a sliding window

At most `max_concurrency` tasks run at once, and the next starts as soon as one ends. **A task awaiting a reviewer
gives its slot back** before it waits: a slot is for work, and waiting is not. The wave is the only dispatcher, so it
needs no semaphore: every worker says what it has to say in the wave's next slot word (its slot came back, then it is
over), and the wave reads them in order. `task_completed` stays in completion order and the results in dispatch
order. **This replaces ADR 032's "every task of a batch is reported before the next batch starts"**: there are no
batches. A DAG crew runs each ready wave the same way.

### Outbound TLS is at most 64 in flight

`src/tls_permit.cyr`: every outbound https exchange takes one of 64 permits and gives it back as sandhi returns. That
covers the hoosh chat and catalog calls, a guarded fetch (per hop, as a redirect may change the scheme), the AGNOS
tools, OTLP export and the load tester. Plain http takes none.

- **A permit covers the whole exchange**, as sandhi handshakes and exchanges in one call. So past 64 at once, an
  https exchange waits for another to end, which caps throughput to an https gateway too.
- **The load tester takes one only for a request that will connect**, as its users keep their connections: a permit
  per request would hold 100 users to 64 for the whole run.
- **A wait that runs out does not fail the exchange.** After 10 seconds it goes ahead without a permit, as every
  exchange did through 2.4.0, warns, and is counted. The worst the cap can do is what there was before it.

It goes once sigil waits for a lane.

### Budgets subdivide: tenant, crew, task

A `BudgetTracker` may have a parent. **What is recorded into a tracker is recorded into every tracker above it, and a
check fails on the nearest one that is spent**, so a task never spends more than its crew has left, nor a crew more
than its tenant has. A tracker's scope names it in a breach: `crew token budget exceeded: 4300 used, 4290 limit`.
Records are `atomic_fetch_add`, where 2.4.0's plain fields could lose a concurrent update.

- **The orchestrator's `max_tokens` and `max_cost_usd` are per crew**, as the Rust line's `BudgetTracker::new` read a
  `ResourceBudget` for one running crew. `agnosai_crew_with_budget` and `agnosai_task_with_budget` only tighten them.
  A crew with nothing to hold it to holds no tracker at all.
- **Every model call is checked first, and what it cost recorded after.** ⚠ The check is outside
  `agnosai_with_retry_a`, because the retry wrapper repeats any error whose text holds `429`, `502` or `503`, and a
  refusal's numbers can. A failed attempt records nothing, so a check between retries could not answer differently.
- **A crew whose budget is spent stops** at its next loop head, as it does for its deadline, and a call the budget
  refused fails its task with the breach as its error. The crew ends `failed`, `reason: budget_exceeded`, keeping the
  results it has: that work was paid for. A crew that spends its budget on its last call has completed.
- **A task's own limit is that task's failure.** The crew runs on, and fails `task_failed` if that task was needed.
- ⚠ **The limit is a ceiling checked before each call, not a reservation.** Calls already in flight each passed the
  check, so a crew can pass its limit by what those calls spend, at most one call each.
- The 2.4.3 delegated worker is the next level down, and holds at most what its manager has left.

### Tenants

An orchestrator given a `TenantRegistry` (`agnosai_orchestrator_with_tenants`) admits a crew that names a tenant
(`agnosai_crew_with_tenant`) only if the registry holds it, it has not spent its tokens or its cost, and it has fewer
than its `max_concurrent_crews` admitted and not ended. A tenant's spending is one tracker across all its crews, above
each crew's, until its operator resets it (`agnosai_orchestrator_reset_tenant`). Registered again, it keeps what it
spent under its new limits.

- ⚠ **Spent means reached**, as a crew's budget does: a tenant at exactly its limit is refused. The Rust line's
  `check_tenant_budget` passes at the limit, and stays as it was, but admitting a crew there only starts one whose
  first call is refused.
- **Concurrency is refused, not queued**, as the Rust line's check returns `ConcurrencyExceeded`. The count and the
  check share the orchestrator's lock, so two crews cannot both take a tenant's last place.
- A tenant is not part of a crew's definition, so its JSON does not carry it, as it does not carry its trace parent.
  An orchestrator with no registry treats a tenant as a name only.

### A refused crew never registers

A crew of more tasks than `agnosai_orchestrator_with_max_tasks` allows (none by default), or one its tenant cannot run,
is refused before it registers: no state, no events, no channel. `agnosai_orchestrator_run_crew_err` and the new
`agnosai_orchestrator_submit_crew_err` return 0 and the reason, and the refusal is logged and audited as
`crew_refused`, the only trace it leaves.

### The tool loop's limits are per crew

`agnosai_crew_with_tool_limits` sets a crew's turns, calls per turn, and bytes of one result and of all of a task's
results. Each only tightens what `agnosai_tool_loop_set_limits` set for the process, and is read as the lower of the
two at every use, so the process's can still move under a crew that set none of its own.

### On the wire

`POST /api/v1/crews` takes `budget: {"max_tokens": N, "max_cost_usd": X}` on the crew and on each task, either key
optional, as a crew spec's and a task's JSON now carry it when set. A budget that cannot be read refuses the request.
`budget_exceeded` joins ADR 032's reasons, and the crew span's `error.type`.

## Consequences

- **Deliberate divergences from the Rust line**: FIFO turns where tokio's semaphore made none; a tenant refused at
  its limit; the sliding window; the TLS cap; per-crew tool limits; the `budget` fields. The REST shapes gain fields
  and lose none.
- **Positive.**
  - A process runs a bounded number of crews, whoever submits them, and a burst queues in order instead of starting
    every crew at once.
  - A crew, a task and a tenant each stop at a ceiling an operator sets, and say which one stopped them.
  - A slow task, or one waiting on a reviewer, no longer holds its siblings back.
  - A burst of https exchanges no longer fails a third of its handshakes.
- **Negative, behaviour changes.**
  - **The 11th crew waits.** A server, and agnostic with its 10, now runs ten crews at once and queues the rest, where
    2.4.0 ran them all. A queued `POST /api/v1/crews` holds its pool worker while it waits, as a running one does.
  - **An https gateway sees at most 64 exchanges at once** from one process.
  - **A parallel crew's events interleave**: a slot is refilled as soon as it frees, so a `task_started` can come
    between two `task_completed`, which batches never did, and a fast task ends before a slow one started ahead of it.
  - **A crew can now fail with `budget_exceeded`**, and a crew refused at admission returns 0 where it used to run.
- **Neutral.** The server has no tenants: who a request runs for needs authentication to say, and is not this
  release's. Its 1,000-task request cap stays the route's own check.

## Alternatives considered

- **A semaphore per wave.** The wave already counts what it started and what it heard end, so a permit there would
  only repeat the count, and every worker would block on it in its own thread.
- **A semaphore that lets a newcomer take a free permit** (tokio's `try_acquire` race, a plain counter). Simpler, and
  a busy orchestrator could pass over a waiting crew for good.
- **Holding a task's slot through its review.** It keeps `max_concurrency` a bound on threads, at the cost of a crew
  that cannot move while its reviewer is away.
- **The budget check inside the retry callback.** It is the one place every attempt passes, and it would have retried
  a refusal whose message held `429`.
- **A reservation per call.** It would close the overshoot, but a call's cost is known only after it, and a
  reservation would refuse calls that would have fitted.
- **Failing an exchange that waited too long for a TLS permit.** The cap is a stopgap for sigil's bug, and a failure
  it invented would be worse than the one it prevents.
- **Queuing a tenant past its concurrency.** A tenant's limit is its operator's, and a queue would hide that it was
  reached; the orchestrator's own limit queues, as it is a resource.
