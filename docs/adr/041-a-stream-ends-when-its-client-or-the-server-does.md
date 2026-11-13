# 041 — A stream ends when its client leaves or the server stops, and a stop drains

**Status**: Accepted, shipped in 2.6.1
**Date**: 2026-11-13

## Context

A stream holds one of the pool's workers for as long as it streams
([ADR 014](014-sse-stream-holds-a-pooled-worker.md)). Before 2.6.1 nothing ended one early:

- **The crew stream ignored its client.** `src/server/routes/sse.cyr` discarded the result of all six
  of its writes and slept between polls. A client that left was noticed by nothing, and the worker
  stayed held until the crew ended, however long that was. The A2A streams of 2.6.0 read their writes'
  results but slept too, so they noticed a departure only at their next write, as late as a
  keep-alive 15 seconds on.
- **A stop drained nothing.** sandhi's stop closes the handoff channel and the listener and returns
  at once ([ADR 013](013-graceful-shutdown-via-signalfd-and-stop-flag.md)), and `main` then
  exited the process. Every request in flight was cut short, and every stream with it. ADR 013 named
  the fix as sandhi exposing its workers so `main` could join them; sandhi 1.13.7 has no such call.

hyper learns that a client left when a write fails, and axum's graceful shutdown waits for every
connection it has handed off. The port has no runtime and no futures, so a stream and a stop both
have to ask for themselves.

## Decision

**A stream waits on its connection instead of sleeping, and ends when its client hangs up or the
server stops. A stop then waits, for a grace at most, until no request is in flight.** Both are in
`src/shutdown.cyr`, a port-local module beside the stop flag, which moved there from
`src/server/serve.cyr` as the streams that read it are included before that file.

- **The wait.** `agnosai_stream_wait(cfd, ms)` is `ppoll` on the connection for `POLLRDHUP` alone,
  for the interval the stream used to sleep. It answers *gone* on the peer's FIN, a reset or an
  error, *stopping* when the stop flag is set, read before and after the wait, and *waited*
  otherwise. Data does not wake it: bytes a client sends after its request neither end the stream
  nor turn the wait into a busy loop. A half-close counts as leaving, as nginx counts it for a
  proxied request. On a file or a pipe, which the suites stream into, there is no hang-up to report
  and the wait is the sleep it replaced.
- **The writes.** Every write of the crew stream is read, and a failed one ends the stream at once.
  After a failure nothing more is written, its last chunk included, as the connection takes no more.
- **The stop ends a stream whole.** A stream that sees the stop sends its last chunk and returns, so
  the drain is not held by a crew it cannot finish, and the client, an `EventSource` by itself,
  reconnects to a server that is not stopping. The crew is not touched: it is the orchestrator's.
- **The count.** The serve handler counts a request in flight from the moment it has the request
  until it has answered it, around the whole of it, so every return is counted out
  (`_agnosai_serve_enter` and `_leave`, atomic).
- **The drain.** Once `sandhi_server_run_pooled` returns for a stop, `agnosai_serve` waits until
  nothing has been in flight for `AGNOSAI_SHUTDOWN_QUIET_MS` (50 ms) together, for
  `AGNOSAI_SHUTDOWN_GRACE_SECS` at most, and logs what it found. The quiet period is there because a
  connection sandhi accepted and queued is not in flight until a worker has read its request, and a
  worker takes the next one microseconds after it answers the last: a drain that ended at the first 0
  would end the process under a request about to start.
- **The grace** is 25 seconds unless `AGNOSAI_SHUTDOWN_GRACE_SECS` says otherwise, so that under
  Kubernetes' default 30-second grace period the drain has finished, its log line is written and the
  spans are flushed before the SIGKILL. 0 waits for nothing, the stop as it was.
- **A second signal ends the wait.** The signal thread reads on after the first, and a second SIGINT
  or SIGTERM sets a force flag the drain reads at once, as an operator pressing Ctrl-C twice means.

## Consequences

- **Positive**: a client that leaves frees its worker within one poll, 50 ms at most, where it held
  it for its crew's life. A stop answers what it was already answering, and a rolling deploy no
  longer cuts requests short.
- **Positive**: `tests/server_live.tcyr` runs the server as the binary does, `agnosai_serve` on a
  loopback port with pooled workers, and pins both: a request in flight at the stop is answered whole
  and the loop returns only after it, and a stream open at the stop ends whole within a poll.
- **Negative**: **what is drained is requests, not crews.** A crew submitted to run on its own
  (`returnImmediately`, the `delegate` tool) stops with the process, and resumes at the next start only
  when `AGNOSAI_STATE_DIR` keeps its log ([ADR 036](036-a-crew-keeps-a-log-and-resumes-from-it.md)).
- **Negative**: **a connection accepted but not yet read is not waited for** past the quiet period.
  Its client has sent no complete request by then, and is closed as any server's stop point closes
  one.
- **Negative**: a blocking request that outlasts the grace is still cut short, and the drain says how
  many were.
- **Neutral**: the stop flag is process-wide, as it was: a library consumer that runs two servers
  stops both.

## Alternatives considered

- **Join sandhi's workers.** What ADR 013 named. sandhi does not expose them, and joining would also
  wait for every stream, which no grace bounds.
- **Detect a departure on the next write only.** Correct, and as late as the next event or
  keep-alive, 15 seconds for an idle crew: the worker would stay held that long for nothing.
- **Count at accept.** The count would cover queued connections too, but the accept loop is sandhi's,
  and agnosai sees a connection only once its request has been read.
- **Leave streams to the grace.** A stream lasts its task's life, so every stop with a stream open
  would wait out the whole grace and then cut it anyway.
