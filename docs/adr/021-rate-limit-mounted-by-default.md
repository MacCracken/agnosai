# 021 — `rate_limit` is mounted by default

## Status: Accepted (2026-08-13)

## Context

`src/server/rate_limit.cyr` is a faithful port of
`rust-old/src/server/rate_limit.rs`, tested since bite 14, and — like the oracle
— **it was never installed**. `rust-old/src/server/mod.rs:18` is
`pub mod rate_limit;` and `mod.rs:47-99` never adds
`rate_limit_middleware` to the router. The port matched that, with
`agnosai_serve_with_rate_limit` as an explicit opt-in.

⚠ **The oracle defines no default rate.** `rate_limit.rs` contains no constants
at all — no limit, no window — because nothing ever constructs the state. So
there is no oracle threshold to inherit; any number is agnosai's.

Two facts made the default worth revisiting:

1. **An invalid token costs a full RSA verify.** `src/server/auth.cyr:597`
   records, deliberately, that signature verification runs *before* the `alg`
   check — the ordering is safe because the path always verifies RSA, and the
   `alg` check is defence-in-depth against a validly-signed token declaring
   another algorithm. The consequence is that a garbage bearer token is not
   cheap: **1.202 ms per verify, ~830/sec/core**. An unauthenticated flood costs
   real CPU.
2. **The failure mode of the default is asymmetric.** An operator who never
   reads the docs and ships unlimited is trivially floodable. An operator who
   wanted the oracle's exact wire and got a limit sees 429s under a load no
   ordinary client generates, and can turn it off with one variable.

⚠ **A prerequisite defect had to be fixed first, and it would have made this
change actively harmful.** `agnosai_serve_handler` called
`agnosai_rate_limit_client_key(str_from(""), 0, str_from(""), 0)` — both
`has_*` flags hardcoded to 0 — so the key was **always `"unknown"`** whatever
headers arrived, and every client shared **one bucket**. The oracle reaches that
single-bucket value only as a *fallback* when neither header is present; the port
made it the only behaviour. Mounting a shared bucket by default would mean one
noisy client returning 429 to everyone, `/health` included — a self-inflicted
outage sold as a safety feature. The handler now reads `X-Forwarded-For` and
`X-Real-IP` through `sandhi_server_find_header{,_a}` and passes them, matching
`extract_client_key`.

## Decision

**Mount rate limiting by default**, at **100 req/s per client key with a burst of
200**, overridable by `AGNOSAI_RATE_LIMIT` (whole requests/second) and
`AGNOSAI_RATE_LIMIT_BURST`. **`AGNOSAI_RATE_LIMIT=0` disables it entirely** and
restores the oracle's exact wire.

The numbers are chosen to be invisible to a legitimate client and still blunt a
flood: 100/s sustained is far above any UI poll or agent tool-call rate, and far
below what an attacker needs to make the verify path hurt. They are agnosai's
numbers and this ADR is where that is recorded.

## Consequences

- **A deliberate wire divergence from the parity oracle**, recorded here as
  CLAUDE.md requires. It joins [ADR 007](007-audit-redirect-revalidation.md),
  [ADR 009](009-auth-constant-time-secret-compare.md),
  [ADR 010](010-jwt-require-configured-iss-aud.md) and
  [ADR 020](020-output-filter-wired-into-task-output.md) — all cases where the
  port is deliberately stricter than the Rust it came from.
- **Clients that were fine before can now see 429**, above 100 req/s from one
  key. That is the point, and the escape hatch is one variable.
- ⚠ **Per-key limiting does not stop a rotating-source flood**, and this ADR does
  not claim it does. It bounds a single misbehaving or compromised client; it is
  not a defence against a distributed attacker, and the RSA-verify cost per
  invalid token is unchanged for the first request from each new key.
- ⚠ **The key is only as trustworthy as the proxy in front.** `X-Forwarded-For`
  is client-settable when nothing strips it, so a direct-to-internet deployment
  lets an attacker mint a fresh bucket per request by varying the header. This
  is the oracle's own key derivation and is kept for parity; behind a proxy that
  overwrites the header it is correct, and in front of one it degrades to
  no-limiting rather than to over-limiting. Deployments exposed directly should
  set `AGNOSAI_RATE_LIMIT=0` and limit upstream, or run behind a proxy.
- Buckets are evicted after `AGNOSAI_RL_EVICTION_SECS` (300) idle, so key
  cardinality is bounded in time rather than growing without limit.
- If the oracle ever installs its own middleware, revisit against whatever rate
  Rust picks and narrow the divergence to whatever remains.
