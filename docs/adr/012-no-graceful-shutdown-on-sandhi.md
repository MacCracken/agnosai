# 012 — The server installs no signal handler and does not drain on shutdown

## Status: Superseded by [013](013-graceful-shutdown-via-signalfd-and-stop-flag.md)

**Superseded 2026-08-03, the same day it was written**, which is unusual enough
to explain. This ADR's own Consequences section named the single upstream change
that would reverse it. agnosai filed that change; **sandhi 1.9.9** implemented
it and **cyrius 6.5.6** vendored it — so the decision below went from correct to
obsolete within hours.

**It is retained, not deleted**, because what it records is still true and still
load-bearing: *why* the capability was absent, and — the part that took the
longest to establish — that the blocker was **not** missing signal support. Two
earlier documents asserted it was, and both were wrong. A future reader hitting
a similar "we can't shut down" wall should read the diagnosis here before
assuming it is theirs.

See [ADR 013](013-graceful-shutdown-via-signalfd-and-stop-flag.md) for what
agnosai actually does now.

⚠ **Every `lib/` citation below is against the superseded bundle** — sandhi
1.9.8, pre-cyrius-6.5.6 — and most of them will **not** resolve in today's
`lib/`. `lib/sandhi.cyr` is 1.9.9 there, the accept loop excerpt has moved and
now carries a stop check, and the stop-facility grep that returned zero now
returns matches. The syscall wrappers are also spelled `sys_sigprocmask` and
`sys_signalfd` (verified 2026-08-11 at `lib/syscalls_linux_common.cyr:385` and
`:392`); `SYS_RT_SIGPROCMASK` / `SYS_SIGNALFD4` are the syscall *numbers* they
use, not the function names. `signal_ignore` is the one citation still exact
(`lib/syscalls.cyr:98`). The originals are kept verbatim because they are the
evidence that was actually checked at the time — this is a record of a
diagnosis, not a map of the current tree.

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
- **Reversing it needs exactly one upstream change**, and that change now
  **exists**: sandhi **1.9.9** ships
  `sandhi_server_options_stop_flag(opts, ptr)` on all five `sandhi_server_run*`
  loops (2026-08-03). It works the way this ADR predicted — the teardown order
  1.9.8 already shipped was correct for it, closing the handoff channel *before*
  the listen fd so workers' `chan_recv` returns 0 and they exit rather than
  parking. The one thing the prediction missed: a stop flag alone is not enough,
  because blocking `accept` would only re-read it after the *next* connection.
  1.9.9 arms the listen fd with `SO_RCVTIMEO` so `accept` surfaces periodically,
  and the resulting EAGAIN was already classified as a non-counting retry.

  ✅ **Vendored in cyrius 6.5.6 and consumed** — see
  [ADR 013](013-graceful-shutdown-via-signalfd-and-stop-flag.md).

## Related

- [ADR 004 — concurrency model](004-concurrency-model.md): why the server is
  `sandhi_server_run_pooled` in the first place.
- [ADR 013](013-graceful-shutdown-via-signalfd-and-stop-flag.md) — what
  superseded this, and the three load-bearing orderings the implementation
  turned out to need.
