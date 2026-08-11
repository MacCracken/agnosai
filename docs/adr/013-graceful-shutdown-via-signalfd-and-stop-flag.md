# 013 — Graceful shutdown via `signalfd` + sandhi's stop flag

## Status: Accepted — supersedes [012](012-no-graceful-shutdown-on-sandhi.md)

## Context

[ADR 012](012-no-graceful-shutdown-on-sandhi.md), written 2026-08-03, recorded
that agnosai ships **no** graceful shutdown, and — importantly — that the reason
was *not* missing signal support. Signal receipt was always available
(`sys_signalfd`, `sys_sigprocmask`). The blocker was one layer down: a
`sandhi_server_run*` loop could not be made to return. It read no flag, its only
exit was a fatal accept, and its listen fd was a loop-local `var` that was never
published — so a handler could fire perfectly and the acceptor would still sit
in `accept` forever.

That ADR named the single upstream change that would reverse it. agnosai filed
it; **sandhi 1.9.9** implemented it the same day, and **cyrius 6.5.6** vendors
that sandhi into `lib/sandhi.cyr`. The blocker is gone.

Two other 6.5.6 items landed from the same filing pass and are used here:
`sys_exit_group` (`lib/syscalls_linux_common.cyr:176` as of the 6.5.18 bundle —
this ADR originally cited `:155`, which was its 6.5.6 line), which replaces the
hand-rolled `syscall(SYS_EXIT_GROUP, …)` in `src/main.cyr`, and
`async_await_readable_ms`, which is not used by agnosai but removed the reason
sandhi's cooperative loop had to poll.

## Decision

**Install a `signalfd`-based SIGINT/SIGTERM handler in `main`, and let it set
the stop flag every sandhi serve loop polls.** This restores parity with the
oracle's `with_graceful_shutdown(shutdown_signal())`
(`rust-old/src/main.rs:130`, `:137-153`).

The shape, all in `src/server/serve.cyr`:

- `_agnosai_serve_stop` — a process-global `i64`. sandhi only ever *reads* it,
  so a plain `store64` from the signal thread is sufficient: an aligned 8-byte
  store is single-copy-atomic on every target here, with nothing to tear and no
  ordering to establish. This deliberately does not pull in `lib/atomic.cyr`.
- `agnosai_serve_mount` wires it with `sandhi_server_options_stop_flag`
  **unconditionally**, on both entry points.
- `agnosai_serve_install_signals()` blocks SIGINT|SIGTERM, opens a signalfd, and
  parks a thread on it.
- `agnosai_serve` now returns **0 for a requested stop** and **1 for a failure**,
  where before 6.5.6 a return was always fatal.

**Three orderings are load-bearing**, and each is a real failure if inverted:

1. **Block the signals before spawning workers.** `sys_sigprocmask` sets the
   *calling thread's* mask and a new thread inherits its creator's. Installed
   after `run_pooled`, the pool would already be running unblocked and SIGTERM
   would kill a worker mid-request instead of draining. `main` therefore calls
   `agnosai_serve_install_signals()` before `agnosai_serve`.
2. **Block before creating the signalfd**, or the signal is delivered
   conventionally and terminates the process rather than becoming readable.
3. **sandhi closes the handoff channel before the listen fd** on the stop path —
   its own teardown order, unchanged from the fatal path. That is what makes
   workers' `chan_recv` return 0 so they exit, rather than parking on a channel
   nobody will ever feed.

**Installing signals is not folded into `agnosai_serve`.** The suites call
`agnosai_serve` with an unbindable address to prove a failed bind returns 1, and
they should not each leave a process-wide signal mask and a parked thread
behind. `main` installs; tests do not.

**A failed install warns and boots anyway.** A server that cannot drain is
strictly better than no server, and the oracle has no corresponding refusal.

## Consequences

- **`./build/agnosai` now stops accepting on SIGINT and SIGTERM**, logging
  `"received shutdown signal, draining"` then `"server shut down gracefully"` —
  the oracle's line, which until now had nothing true to report. Verified live:
  both signals exit **0** in ~100 ms, and a request racing the shutdown still
  completes **200**. ⚠ **Read that last result narrowly.** It completes because
  the stop is only noticed at the next SO_RCVTIMEO poll, so a short request has
  up to `SANDHI_SERVER_STOP_POLL_MS` (100 ms) of ordinary serving left — not
  because anything waits for it. See the in-flight bullet below.
- **`agnosai_serve`'s return value gained a third meaning.** 0 = asked to stop,
  1 = failed. Callers written against the old contract still behave correctly,
  because the failure value is unchanged — but anything treating *any* return as
  fatal now over-reports.
- **In-flight requests are not drained, and the binary is stricter about that
  than this ADR originally said.** The stop is "stop accepting", not "wait for
  quiescence".

  What sandhi promises is that a worker already holding a connection *may*
  finish it: the stop path closes the handoff channel and the listen fd and
  returns, without killing workers (`lib/sandhi.cyr:14334-14337` — verified
  2026-08-11), and `idle_ms` bounds how long a worker can be held.

  **`./build/agnosai` does not give the worker that chance.** Nothing joins the
  pool. `agnosai_serve` returns 0, `main` logs, calls `agnosai_telemetry_shutdown`
  (which returns immediately unless OTLP is configured) and returns, and the
  epilogue calls `_agnosai_exit_process` → `sys_exit_group` (`src/main.cyr:279-285`,
  `:390-397`, epilogue `:405-406`). Every worker dies where it stands, microseconds after the acceptor
  returned. A request that has not finished inside the ≤100 ms poll window is
  severed, and so is an SSE stream ([ADR 014](014-sse-stream-holds-a-pooled-worker.md)),
  which by construction never finishes on its own.

  **This is a residual divergence from the oracle, not parity.** axum's
  `with_graceful_shutdown` does await connections it has already handed off; the
  original wording here claimed a match that the process-level exit does not
  deliver. Closing it needs sandhi to expose the worker handles (or a join/
  quiesce call) so `main` can wait before `exit_group` — an upstream ask, not a
  local fix, and not filed as of 2026-08-11.
- **Every server pays SO_RCVTIMEO on the listen fd**, because the flag is wired
  unconditionally — about ten wakeups a second on an idle listener. Measured as
  noise against the accept path, and cheaper than a second code path to avoid
  it. Shutdown latency is bounded by that poll interval.
- **Signal disposition is now process-wide state set by a library call.**
  `agnosai_serve_install_signals` blocks two signals for every thread created
  afterwards. That is the correct behaviour for a server binary and a
  surprising one for a library consumer, which is why it is opt-in rather than
  automatic.

## Related

- [ADR 012](012-no-graceful-shutdown-on-sandhi.md) — superseded; retained
  because it records *why* the capability was absent and what was filed.
- [ADR 004](004-concurrency-model.md) — why the server is
  `sandhi_server_run_pooled`.
