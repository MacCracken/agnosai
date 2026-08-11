# ADR-004: Concurrency Model

## Status: Accepted for `rust-old/` — re-decided by the port

**Date**: 2026-03-18 (Rust era). **Amended 2026-08-11.** Every row of the
original decision table names a tokio or `dashmap` type. **The Cyrius port uses
none of them** — there is no async runtime in `src/`, and concurrency is OS
threads plus explicit mutexes. The Rust-era table is kept below because it is
still the right reading of `rust-old/`, which is the parity oracle; the port's
model follows it.

## Context

Agnostic v1 (Python/CrewAI) is limited by the GIL — concurrent crew execution is serialized. AgnosAI needs real concurrency for multi-agent orchestration.

## Decision (Rust era — `rust-old/`)

Use patterns proven in Agnosticos:

| Pattern | Where | Why |
|---------|-------|-----|
| `Arc<RwLock<OrchestratorState>>` | Orchestrator | Single lock for compound operations; readers dominate |
| `tokio::sync::broadcast` | Pub/sub | One-to-many event delivery with backpressure |
| `DashMap` | Tool registry, subscriptions | Lock-free concurrent reads; high read:write ratio |
| `tokio::Semaphore` | Rate limiting, parallel execution | Bounded concurrency without busy-waiting |
| `tokio::task::JoinSet` | Parallel/DAG crew execution | Concurrent task execution with result collection |
| Priority `VecDeque` per level | Task scheduler | O(1) enqueue/dequeue per priority tier |

Rationale as recorded at the time: these patterns were battle-tested in
Agnosticos; `RwLock` over `Mutex` because orchestrator state is read far more
than written; `DashMap` over `RwLock<HashMap>` for registries with concurrent
lookups; `broadcast` because subscribers need independent receivers.

## Decision (the port) — OS threads, no async runtime

**tokio → `sandhi_server_run_pooled` + one OS thread per unit of work.** The
reasoning is in `docs/development/cyrius-port-plan.md`, under *"The concurrency
decision"*, and it is not a language limitation — Cyrius has `lib/async.cyr`, and
it was rejected on purpose. That reactor requires poll-structured, re-entrant
handlers ("resuming FROM THE TOP when woken"), which would mean hand-rewriting
every agnosai handler as a state machine; and sandhi's own `run_async` does a
blocking recv internally, giving **zero** handler concurrency — for handlers that
block for seconds on a hoosh inference call, a 128-connection batch serialises to
minutes. `lib/async.cyr` stays in scope for client-side fan-out only.

Row by row, verified against the tree:

| Rust-era row | What the port does | Where |
|---|---|---|
| `Arc<RwLock<OrchestratorState>>` | Registry behind one plain mutex (`AGN_OR_MUTEX`); a crew runs on the **calling** thread, so the concurrency bound is the caller's, not a semaphore's | `src/orchestrator/orchestrator.cyr` |
| `tokio::sync::broadcast` | `agnosai_chan_push_lossy` — evict-oldest on a full ring, built on `chan_try_send`. A pattern maps to a **vec of per-subscriber channels**, because Cyrius channels are single-consumer FIFOs | `src/chan_lossy.cyr`, `src/orchestrator/pubsub.cyr`, `src/server/sse.cyr` |
| `DashMap` | Hashmap behind a futex mutex. **The lock is mandatory, not optional** — every pooled worker is its own OS thread, so an unguarded `map_set` during a concurrent `map_get` is a live data race | `src/tools/registry.cyr` |
| `tokio::Semaphore` | No semaphore exists. Parallel crew execution bounds itself by **batch size**; inbound HTTP rate limiting is a majra token bucket (and is not mounted by default) | `src/orchestrator/crew_runner.cyr`, `src/server/rate_limit.cyr` |
| `tokio::task::JoinSet` | `thread_create` in batches of `max_concurrency`, every thread `thread_join`ed before the next batch — so nothing is left detached and the batch size *is* the limit | `_agnosai_crew_run_wave`, `src/orchestrator/crew_runner.cyr` |
| Priority `VecDeque` per level | Five vecs indexed by priority (`AGNOSAI_SCHED_TIERS = 5`) for the task scheduler; majra's `cpq_*` for the LLM inference queue | `src/orchestrator/scheduler.cyr`, `src/llm/inference_queue.cyr` |

Two things the original table had no row for, because tokio made them invisible:

- **The HTTP transport.** `agnosai_serve` is `sandhi_server_run_pooled`
  (`src/server/serve.cyr`). One OS thread per worker, one reusable buffer each,
  and a per-request arena with a single `reset_via` exit path (port-plan blocker
  #3). An SSE stream holds one of those workers for its whole life —
  [ADR 014](014-sse-stream-holds-a-pooled-worker.md).
- **`MAX_CONCURRENT_REQUESTS`.** The oracle's tower `ConcurrencyLimitLayer`
  *queues* requests above the limit; sandhi's pool bounds work in flight and does
  not queue. The constant (100) is carried in `src/server/router.cyr` so the
  worker pool can be sized against the oracle's intent rather than an invented
  number, and so the divergence is stated rather than dropped.

Where the port's substitution is genuinely *stronger* than what it replaces, that
is noted at the call site rather than here — `agnosai_inference_queue_enqueue`
being synchronous is the clearest case (`src/llm/inference_queue.cyr`).

## Consequences

- **`Send + Sync` has no Cyrius equivalent.** The compiler will not catch a
  shared mutable structure reached from two threads. Safety is by construction —
  an explicit mutex on every structure a pooled worker can touch — which makes
  the "every entry point takes the lock" comment in `tools/registry.cyr` a
  correctness requirement, not documentation.
- Lock ordering must still be documented to prevent deadlocks as the system
  grows. Nothing in the tree currently takes two of these locks at once.
- **Bounded rings still lose messages for slow subscribers**, and the capacity is
  still 256 — `AGNOSAI_PUBSUB_CHANNEL_CAPACITY` and
  `AGNOSAI_SSE_CHANNEL_CAPACITY`, both matching the oracle's `CHANNEL_CAPACITY`.
  The loss is *counted*: `agnosai_event_sub_lagged` reports what a lagging reader
  missed, which is the port's stand-in for tokio's `RecvError::Lagged(n)`.
- **A blocking call blocks a real thread**, which is why `agnosai_with_retry`'s
  driver can simply `sleep_ms` where the oracle awaited a timer.
- **Thread creation is not free and is not unbounded.** `_agnosai_crew_run_wave`
  falls back to running a job inline when `thread_create` fails, rather than
  dropping it — a missing result would silently shorten the crew's output.
