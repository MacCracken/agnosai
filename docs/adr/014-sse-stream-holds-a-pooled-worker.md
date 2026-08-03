# 014 — An SSE stream holds a pooled worker for its whole life

## Status: Accepted

## Context

`GET /api/v1/crews/{id}/stream` streams a crew's events until the crew ends.
The oracle is `rust-old/src/server/routes/sse.rs`.

The question this ADR settles is **how many concurrent streams the port can
serve**, because the answer differs from the oracle's and the difference is not
obvious. It is not obvious in a specific and dangerous way: there is a plausible
argument that the two are equivalent, and that argument is **wrong**.

### The wrong argument, written down so it is not re-derived

`rust-old/src/server/mod.rs` installs `tower::limit::ConcurrencyLimitLayer::new(100)`.
`src/server/serve.cyr` carries that across as `AGNOSAI_SERVE_WORKERS = 100`. It
is therefore tempting to conclude: *an SSE stream holds a permit in the oracle
just as it holds a worker here, so 100 streams exhaust both, and there is no
divergence to document.*

That is false. tower holds its permit in `ResponseFuture`, which is dropped when
the inner service future **resolves to a response** — not when the response
*body* finishes. And `crew_stream` (`routes/sse.rs:17-92`) is an `async fn` with
**zero `.await` before it returns**:

- `state.events.has(id)` — synchronous
- `state.events.subscribe(id)` — synchronous
- `state.events.remove(id)` — synchronous
- `async_stream::stream! { ... }` — builds a **lazy** stream; the body does not
  run until polled
- `Sse::new(stream).keep_alive(...)` — returns immediately

So the future resolves in microseconds, the permit drops, and the SSE body is
streamed afterwards by hyper's per-connection task, entirely outside the
semaphore. **The oracle's concurrency limit bounds request *handling*, not
response *streaming*, and it therefore serves effectively unbounded concurrent
SSE streams.**

### What the port can actually do

There is no async runtime. `sandhi_server_run_pooled` hands an accepted
connection to one of `AGNOSAI_SERVE_WORKERS` OS threads, and that worker is
occupied until the handler returns. `agnosai_sse_crew_stream` returns when the
crew ends — which for a long-running crew is minutes, and for a crew that never
ends is never.

So a stream costs one of 100 workers for its whole life, and 100 concurrent
streams leave **zero** workers for ordinary requests. `/health` stops answering.

## Decision

**Ship it, with the cost documented and no invented cap.**

Concretely: `agnosai_sse_crew_stream` occupies its worker, and nothing in the
port limits how many of the 100 may be streams.

Rejected alternatives:

- **Cap concurrent streams at some fraction of the pool** (say 20), answering
  503 above it. Rejected: the number would be invented. The oracle has no such
  limit and returns no such status, so every client that works against the Rust
  build and gets a 503 here is looking at a wire divergence agnosai made up. If
  an operator needs that bound, it belongs in a reverse proxy where the number
  is theirs.
- **Raise `AGNOSAI_SERVE_WORKERS`.** Rejected as a fix, though it is a valid
  knob: it moves the number without changing the shape, and each worker costs a
  10 MiB request buffer plus a 2 MiB stack, so "just make it 1000" is ~12 GiB.
- **Hand the connection to a dedicated streaming thread** so the worker returns
  immediately. This is the *right* long-term answer and is what hyper is doing
  for the oracle. Rejected for this bite: it needs a thread per stream with no
  pool bounding it (trading worker starvation for unbounded thread creation), a
  lifetime for the connection fd that outlives the handler, and a way to stop
  those threads at shutdown. That is a design, not a transcription, and it would
  have doubled the size of the last bite in the milestone.

## Consequences

- **Concurrent SSE streams are bounded by `AGNOSAI_SERVE_WORKERS` (100), and at
  that bound the server stops answering everything else.** The oracle has no
  equivalent ceiling. This is the divergence; it is operational rather than
  wire-level — no request gets a different status or body until the pool is
  exhausted, at which point requests are not served at all rather than served
  differently.
- **A stream that never ends holds its worker forever.** `agnosai_sse_crew_stream`
  exits on the subscription closing (`agnosai_event_bus_remove`) or on the client
  lagging. A crew that runs indefinitely and never lags keeps its worker
  indefinitely — correctly, but at that cost.
- **Deployments that expect many concurrent watchers must raise the pool and
  size memory for it**, or front the endpoint with something that multiplexes.
  This is the one place where the port's capacity story is materially worse than
  the Rust build's, and it should be stated in operator docs rather than
  discovered.
- **Reversing this is a self-contained change**: move the stream onto its own
  thread and return the worker. Nothing outside `agnosai_sse_crew_stream` and
  the interception in `agnosai_serve_handler` would change. The blocker is a
  thread-lifetime and shutdown design, not a missing primitive.

## Related

- [ADR 004](004-concurrency-model.md) — why the server is
  `sandhi_server_run_pooled` rather than an async reactor. This ADR is that
  decision's first sharp edge.
- [ADR 013](013-graceful-shutdown-via-signalfd-and-stop-flag.md) — shutdown
  stops *accepting*; workers holding streams finish on their own terms, so a
  drain with live streams waits on them.
- `src/server/routes/sse.cyr` — the module header carries the short form of the
  tower analysis above.
