# 012 — The server installs no signal handler and does not drain on shutdown

## Status: Accepted

## Context

`rust-old/src/main.rs:130` wraps the listener in a graceful shutdown:

```rust
axum::serve(listener, app)
    .with_graceful_shutdown(shutdown_signal())
    .await?;

tracing::info!("server shut down gracefully");
```

and `shutdown_signal()` (`main.rs:137-153`) selects over `tokio::signal::ctrl_c()`
and a `SignalKind::terminate()` stream, logging which one arrived. On either, axum
stops accepting, lets in-flight requests finish, and returns.

Bite 16 (`src/main.cyr`) is the first thing in the port that actually calls
`agnosai_serve`, so it is the first place this matters.

**The roadmap's A2 note gave the wrong reason for deferring it.** It read
*"Graceful shutdown needs a raw `rt_sigaction`; no signal helper exists in
`lib/`"*, and both halves are false:

- A signal helper **does** exist — `signal_ignore(signum)` at `lib/syscalls.cyr:98`,
  with the `#ifdef` target split already written. It is `SIG_IGN`-only, so it
  cannot install a handler, but it is not absent.
- Signal **receipt** is fully available without any `rt_sigaction`:
  `sys_rt_sigprocmask` (`lib/syscalls_linux_common.cyr:323`, `SYS_RT_SIGPROCMASK
  = 14`) and `sys_signalfd4` (`:330`, `SYS_SIGNALFD4 = 289`) are both wrapped.
  Blocking SIGINT/SIGTERM and reading them off a `signalfd` is ordinary code.

The actual blocker is one layer down, and no amount of signal work reaches it:
**there is no way to make sandhi's accept loop return.**

```
# lib/sandhi.cyr:14205-14219
while (1 == 1) {
    var cr = sock_accept(sfd);
    if (is_err_result(cr) == 1) {
        var act = _sandhi_accept_step(ast, payload(cr));
        if (act == SANDHI_ACCEPT_FATAL) { chan_close(ch); sock_close(sfd); return 1; }
        ...
```

Three facts, each verified against the pinned 1.9.8 bundle:

1. The loop **reads no flag**. Its only exit is `SANDHI_ACCEPT_FATAL`, i.e. the
   listener is already structurally dead.
2. The listen fd is the loop-local `var sfd` (`lib/sandhi.cyr:14176`) and is
   **never published**, so a signal handler or a second thread has nothing to
   `sock_close` out of band, and no self-connect nudge is addressable either.
3. `grep` for a stop facility across the whole bundle — `sandhi_server_stop`,
   `sandhi_server_shutdown`, `stop_flag`, `_should_stop` — returns **zero**
   matches.

So a handler could be installed and would fire, and the process would still sit
in `sock_accept` with no way to unwind.

## Decision

**Install no signal handler. Do not claim a graceful shutdown.**

SIGINT and SIGTERM keep their default disposition, which terminates the process
immediately. `src/main.cyr` returns whatever `agnosai_serve` returns and exits
with it; there is no `"server shut down gracefully"` line, because nothing
graceful happens.

The rejected alternatives, so they are not re-proposed:

- **Hand-roll the accept loop in agnosai** so the flag check can live in our
  code. Rejected: it would reimplement `_sandhi_accept_step`'s backoff and
  fatal-classification policy, the bounded handoff channel, and the worker pool
  — the entire substance of `sandhi_server_run_pooled` — to add one `if`. That
  is a fork of the transport tier, not a bite.
- **Install a handler that flips a flag anyway**, so the shape matches the
  oracle. Rejected as the worst option: it reports a capability that does not
  exist. The process would still not drain, and the next reader would believe
  it did. CLAUDE.md's rule is that a Cyrius behaviour diverging *silently* from
  Rust is the worst outcome, and a handler that visibly does nothing is exactly
  that.
- **`signalfd` + a watchdog thread that `sock_close`s the listener.** Rejected
  on fact 2: the fd is not reachable from outside the loop.

## Consequences

- **In-flight requests are not drained.** On SIGTERM the process dies where it
  stands; a request mid-handler is lost, and the client sees a reset. In
  practice a supervisor's stop-then-restart will drop responses that the Rust
  build would have completed.
- **No shutdown log line.** Anything scraping for `"server shut down
  gracefully"` will not find it.
- **The process was always killable and still is.** This removes nothing that
  worked before; it declines to add something that cannot work yet.
- **This is a divergence from the oracle's observable behaviour**, which is why
  it is an ADR rather than a code comment. It does not change the *wire* — no
  request gets a different status or body — only what happens to connections
  already open when the process is asked to stop.
- **Reversing it needs exactly one upstream change**, filed against sandhi: a
  stop facility on the five `sandhi_server_run*` loops, e.g.
  `sandhi_server_options_stop_flag(opts, ptr)` polled in the accept loop, or
  simply publishing the listen fd. The teardown order 1.9.8 already ships is
  correct for it — `lib/sandhi.cyr:14213` closes the handoff channel *before*
  the listen fd, so workers' `chan_recv` returns 0 and they exit rather than
  parking. When that lands, `src/main.cyr` gains a `signalfd` read and this ADR
  is superseded, with no change to any other module.

## Related

- [ADR 004 — concurrency model](004-concurrency-model.md): why the server is
  `sandhi_server_run_pooled` in the first place.
- `src/server/serve.cyr` — `agnosai_serve`'s doc records that a return is fatal
  rather than a clean exit, which is the same fact from the caller's side.
