# 022 — Crew events and registry status say what happened

## Status: Accepted (2026-10-04), shipped in 2.1.5. Proposed 2026-10-03.

Note (2026-10-03): [ADR 023](023-genai-semconv-spans-and-w3c-trace-context.md),
in the same release (2.1.5), gave `agnosai_execute_task_in_crew` a trailing
`parent_sc` (the crew's span context) and the hoosh client's chat pointer a
fourth argument of the same kind. The signatures quoted below are as this ADR
introduced them.

## Context

agnostic 0.1.9 built its live crew views on agnosai's event bus and crew
registry. It found six places where what a watcher reads is not what happened
(roadmap **B17**). Each was checked against the 2.1.4 source and against
`rust-old/`. **Every line citation below is as of tag 2.1.4** — the port's,
because this change moves them, and the oracle's, because `rust-old/` is
scheduled for deletion and these lines must stay reachable through git history.

| # | The port at 2.1.4 | The oracle (`rust-old/src/orchestrator/`) |
|---|---|---|
| 1 | A parallel or DAG wave emits `task_started` for every job before its first batch runs (`crew_runner.cyr:1092-1107`). It emits `task_completed` only after its last batch has joined (`:1146-1165`). A job stopped by the deadline or a cancel was announced as started and never completed. | Parallel emits `task_started` for every task up front (`crew_runner.rs:337-348`) and `task_completed` as each task joins, in **completion order**: `while let Some(res) = join_set.join_next()` (`:400-411`) yields whichever task finishes first. DAG does the same per wave (`:488-500`, `:521-535`). **The port's started half is the oracle's; its completed half is a port regression.** |
| 2 | Workers run `execute_task` with `event_tx = 0` (`crew_runner.cyr:756-757`), so parallel and DAG tasks send no `token` event. | The same: `None` at `crew_runner.rs:393` and `:514`. The port's comment (`crew_runner.cyr:1076-1078`) added "two threads racing on one channel". There is no race: `agnosai_event_sender_send` holds the bus mutex across its whole fan-out (`server/sse.cyr:257-276`). |
| 3 | The profile's `agent_cost_usd` is aggregated from a result's `metadata["agent"]` (`crew_runner.cyr:1316-1319`), which nothing writes. Its recorder also **replaces** where it should sum (`core/crew.cyr:339-342`). | It aggregates the same key and sums with `+=` (`crew_runner.rs:186-194`, the `+=` at `:192`). It never writes the key either (success metadata, `:820-841`), so the field was always empty. |
| 4 | `token` events take their crew id from `task.context["crew_id"]`, else the literal `"unknown"` (`crew_runner.cyr:415-427`, `:580-588`). The runner never stamps the context, so every token event says `"unknown"`. The quirk was pinned on purpose by a test. | The same (`crew_runner.rs:786-801`). Every other event carries the spec's id (`emit`, `:113-121`). |
| 5 | The deadline is cooperative, so a timed-out runner still finishes. It computes its status from the partial results — `completed` when they all completed, *including when there are none* — and emits that in `crew_completed` (`crew_runner.cyr:1456-1487`). Only then does the orchestrator store `failed` (`orchestrator.cyr:369-371`). The GenAI crew span can record OK for a timed-out crew (`crew_runner.cyr:1493-1494`). | `tokio::time::timeout` drops the run future (`orchestrator.rs:204-214`), so no `crew_completed` is emitted at all. The stored state is `Failed`, with no results and no profile. |
| 6 | The registry holds `pending` from registration (`orchestrator.cyr:249-250`) until the terminal state replaces it (`:405`). `AGNOSAI_CREW_RUNNING` is parsed and rendered (`core/crew.cyr:79,94`, `server/routes/dashboard.cyr:67`) and never stored. | The same (`orchestrator.rs:168-176`, `:254`). `CrewStatus::Running` appears only in tests (`core/crew.rs:174,193,234`, `durable_state.rs:114`). |

agnostic works around every one of these:

- it infers RUNNING from a collected `crew_started` (its ADR 0009);
- Swarm Command queues the tail of a parallel wave by hand;
- the views take parallel outputs from the final poll.

Roadmap F3 will specify the full lifecycle: the states, the legal transitions
and a per-crew `seq`. These six do not need that design, so they go first.

## Decision

**Every crew event and every registry status a watcher can read describes
something that happened, when it happened, under the crew it happened in.**
Applied to the six:

1. **Wave events.** A parallel or DAG task's `task_started` is emitted when that
   task is dispatched to its worker, which is when its batch starts. Its
   `task_completed` is emitted as soon as its worker is joined, in dispatch
   order. A batch is joined in the order it was dispatched, so a task that
   finishes early is reported after any slower task dispatched before it in the
   same batch. It is never reported later than the end of its own batch: every
   task in a batch is reported before the next batch starts. A task stopped
   before dispatch, by the deadline or a cancel, gets neither event.
   (`_agnosai_crew_run_wave`.)
2. **Worker tokens.** Workers carry the crew's event sender, when someone is
   subscribed, and emit the same single `token` event the sequential path does.
   The job struct gains `AGN_CJ_EVENT_TX` and `AGN_CJ_CREW_ID`.
3. **Agent cost.** An LLM-answered result's metadata carries `agent`, the
   assigned agent's key, and `agnosai_crew_profile_record_agent_cost` sums, as
   the oracle's `+=` does. `agent_cost_usd` is therefore populated whenever the
   gateway reports a cost. Placeholder and error results keep their
   oracle-pinned metadata shapes.
4. **Token crew id.** A `token` event emitted by the runner carries the spec's
   crew id, like every other event: the runner passes it down through the new
   `agnosai_execute_task_in_crew(task, agent, client, event_tx, crew_id)`, and a
   given id wins over `task.context["crew_id"]`. `agnosai_execute_task` called
   outside a runner keeps the old fallback: the context's `crew_id`, else
   `"unknown"`.
5. **Timeouts.** On a timeout the runner itself reports the oracle's timeout
   state: `failed`, no results, no profile. Its `crew_completed` says
   `status: "failed"` and `task_count: 0` — the state's count, not the run's —
   so the event and the stored state agree by construction, and the crew span is
   ERROR. A timeout outranks a cancellation, as the orchestrator's substitution
   already does. Metrics still count the work that ran. The orchestrator's
   substitution stays, for its warning, and is now idempotent.
6. **RUNNING.** The orchestrator stores `running` when it hands a registered
   crew to its runner (`_agnosai_orch_advance`). It moves only from `pending`, so
   a cancel that landed first stands. If the run errors before producing a state
   (a cyclic DAG, or a DAG deadlock), it puts `pending` back, leaving that arm
   storing what it stored before. **That makes `running` → `pending` a real
   transition a poller can observe.** The advance happens before the runner's
   topological sort, so even a cyclic DAG passes through `running`. A DAG
   deadlock reverts only after its earlier waves have run and sent their task
   events: a task depending on an id that is not in the spec is never ready,
   and `agnosai_topological_sort_tasks` skips ids it cannot find
   (`scheduler.cyr:344-356`), so the sort does not catch it.
   `POST /api/v1/crews` reaches neither, since its dependencies are
   range-checked and cycle-checked task indices. An embedding caller that
   builds its own spec can.

**Out of scope, left to F3 or later:**

- an event `seq`;
- an `awaiting_approval` state;
- a reason field on `crew_completed`;
- auditing parallel and DAG tasks (still sequential-only, `crew_runner.cyr:985-989`
  at 2.1.4);
- a terminal state for the DAG error arm;
- `task_completed` in completion order within a batch (see *Alternatives
  considered*).

## Consequences

- **Deliberate divergences from the parity oracle**, recorded here as CLAUDE.md
  requires: both halves of 1, and 2, 3, 4, 5 and 6. In 1, `task_started` goes
  out at dispatch where the oracle sends it up front. `task_completed` moves
  *toward* the oracle in timing, per join rather than after the last batch.
  Its order still differs: dispatch order within a batch, where the oracle's
  `join_next` (`crew_runner.rs:400`, `:521`) reports completion order. This ADR
  joins
  [007](007-audit-redirect-revalidation.md),
  [009](009-auth-constant-time-secret-compare.md),
  [010](010-jwt-require-configured-iss-aud.md),
  [020](020-output-filter-wired-into-task-output.md) and
  [021](021-rate-limit-mounted-by-default.md).
- **Positive:**
  - A watcher can tell which tasks of a wave are running and which are done.
  - Parallel and DAG outputs arrive while the crew runs.
  - Per-agent cost is real.
  - `GET /api/v1/crews/{id}` and the dashboard show `running`.
  - A timed-out crew never announces `completed`, and its GenAI crew span is
    now ERROR.
  - `/stream` token events name their crew.
  - `GET /api/v1/dashboard/agents` lists the agents of LLM-answered crews. It
    reads the same `metadata["agent"]` key (the oracle's `dashboard.rs:56`), so
    it rendered `[]` for every real crew until something wrote it.
- **Negative:**
  - **Event order changes for parallel and DAG crews.** It becomes
    `s0 s1 c0 c1 s2 …` instead of `s0 … sN … cN`. A consumer must not assume
    every `task_started` precedes every `task_completed`.
  - **`task_completed` follows dispatch order within a batch**, not completion
    order. A fast task dispatched after a slow one in the same batch is reported
    after it. The lag is bounded by the batch. A DAG wave runs as one batch
    (`max_concurrency` 0), so in a DAG crew the lag is bounded by that wave's
    slowest task.
  - **Event volume rises** by one `token` event per LLM-answered parallel or DAG
    task, and each carries the task's full output. Bounded queues fill sooner:
    agnosai's 256-event subscriber channels (`AGNOSAI_SSE_CHANNEL_CAPACITY`) and
    agnostic's 256-event ring.
  - **Status can go `running` → `pending`** on the DAG error arm (decision 6).
    A poller must not treat `pending` after `running` as a new run. F3's
    lifecycle owns the terminal state that would close this edge.
  - **Result metadata gains a key.**
  - **`agnosai_crew_runner_run` with a deadline now discards partial results
    itself.** In-tree, only the orchestrator sets a deadline.
  - **The bus mutex**, which is shared by every crew on the bus, now also takes
    sends from worker threads. Each critical section is a fan-out of pointer
    pushes, and a worker gets the sender only when someone is subscribed.
  - **`agnosai_crew_profile_record_agent_cost` changes from set to add.** It is
    public, but its only non-test caller is `_agnosai_crew_build_profile`;
    deserialization builds the map through `_agnosai_cost_map_from_value`. It
    clones the key on every call, as it did before: `map_set` overwrites the
    stored key pointer as well as the value, so writing the sum back under the
    caller's borrowed key would swap the owned clone out for a borrow.
- **agnostic needs no code change to stay correct:**
  - it maps engine RUNNING (`engine/outcome.cyr:170`);
  - its latch accepts non-terminal statuses (`engine/ledger.cyr:647-671`);
  - it picks result metadata field by field (`engine/outcome.cyr:461-515`);
  - its events route does not render an event's `crew_id`
    (`routes/crews.cyr:459-471`).

  Its comments saying the registry never reports RUNNING become stale at the
  re-pin.
- **Testing:** before this change, the LLM success arm could not be reached
  offline. A mutant could unwire 2, 3, 4, or ADR 020's filter on that arm and
  still pass every test. The hoosh client therefore gains a chat function
  pointer (`AGN_HC_CHAT_FP`, `agnosai_hoosh_client_with_chat`). Its default is
  the live call, and `agnosai_hoosh_client_with_chat` is called only by suites —
  the pattern `tools/agnos.cyr` and `llm/inference_queue.cyr` already use.
  Nothing is injected in production. With it:
  - a stub gateway answers `tests/orch_crew_runner.tcyr` and
    `tests/orch_orchestrator.tcyr` offline, and the second reads the registry
    from inside a running crew;
  - a stub that moves the runner's deadline into the past while answering task 1
    gives a **deterministic mid-run timeout**, which is what separates the
    state's `task_count` from the run's;
  - 22 mutants — across the seam, the six parts, and ADR 020's filter on the
    LLM arm — were each killed;
  - "an unwatched bus costs the workers nothing" is checked exactly: a crew run
    with a bus nobody subscribes to must allocate the same bytes as the same
    run with no bus. That kills seven more mutants, one per event gate. Two of
    them, the wave's worker-sender and dispatch-time `task_started` gates,
    survived the first review round, when only the result count of an unwatched
    parallel crew was checked;
  - `task_completed` going out as *its* worker is joined, not at the end of its
    batch, is checked by a stub that holds the second task of a two-wide batch
    until it sees the first task's `task_completed`, with a 5 s bound. A runner
    that joined the whole batch before reporting any of it would still be
    blocked on that second worker, so the wait runs out. The batch-granularity
    checks pass under both shapes, so this mutant survived every test until a
    review found it;
  - a timed-out crew's GenAI span being ERROR is checked in
    `tests/telemetry_wiring.tcyr`, which drains the OTLP ring after a run whose
    deadline has already passed. A mutant that kept a timed-out span OK had
    also survived every test until that review.

  The timeout-plus-cancel precedence is not reachable deterministically — both
  are read at the same loop heads, cancel first — and is stated in the code
  rather than tested.
- **Neutral:** F3 builds its state machine and `seq` on these semantics rather
  than replacing them.

## Alternatives considered

- **Stamp `crew_id` into each task's context instead of passing it.** Rejected.
  `_agnosai_crew_build_request` renders the whole context map into the prompt
  (`crew_runner.cyr:491-538` at 2.1.4), so:
  - every context-free task would gain a context preamble;
  - an internal id would reach the model;
  - the request body, which is hoosh's cache key, would differ per crew.
- **Emit no `crew_completed` on a timeout, as the oracle in effect does.**
  Rejected. Every other way a crew ends produces a terminal event. Consumers
  treat `crew_completed` as the last word: agnostic's Swarm Command finalises on
  it, and its Crews view waits for it after a cancel. A silent channel close
  would be the one exception.
- **Report `task_completed` in completion order, as the oracle does.** Not taken
  in this change. `thread_join` waits on one named thread, so completion order
  needs the workers to signal when they finish: a completion channel the runner
  drains, or each worker emitting its own event. The port also keeps results in
  job order, where the oracle's are in completion order, and that change would
  have to keep or change that order deliberately. It restructures the wave
  beyond B17's six fixes, and
  dispatch order already reports every task of a batch before the next batch
  starts. It is left for F3 or later.
- **Keep `task_started` up front and add a `task_running` event.** Rejected. A
  new event type belongs to F3's lifecycle design, and `task_started` already
  means "this task started" to every consumer.
- **Fill `agent_cost_usd` from a runner-side task-to-agent map and leave result
  metadata alone.** Rejected. The oracle's aggregation reads
  `metadata["agent"]`; writing that key completes the design the oracle
  declared, and also gives each result its agent.
- **Six ADRs.** Rejected. This is one decision — events and status describe
  what happened — applied in six places.
- **Set RUNNING from inside the runner.** Rejected. The runner has no registry
  handle, and giving it one would couple runner to orchestrator for a single
  store.
