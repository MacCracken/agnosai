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

⚠ **Two prerequisite defects had to be fixed first, and either would have made
this change actively harmful — or, in the second case, purely decorative.**

**The limiter refused nothing at all.** `agnosai_rate_limit_check` handed its
key to majra's `ratelimit_check` as a `Str`, but majra keys its bucket map on a
**cstr** — `KeyTypeCstr` hashes with `hash_str` and compares with `streq`, both
reading bytes straight from the pointer. Given a `Str` VALUE, both read the Str
**header**, whose first eight bytes are the data pointer, so identical content
at two addresses hashed two different ways. The key is derived per request, so
**every request got its own bucket with a full burst**: three identical requests
through the handler produced 3 active keys and 0 rejections. Mounting that by
default would have added a per-request map insert, a warning path and an ADR's
worth of divergence in exchange for **no limiting whatsoever** — the worst
outcome, because the wire would look protected. Fixed here by converting through
`_agnosai_rl_cstr_a` (built in the request arena, not the global bump); majra
2.6.4 carries the other half, owning its copy of the key rather than borrowing
one the caller may reuse.

**Every client shared one bucket.** `agnosai_serve_handler` called
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
- ⚠ **The key is only as trustworthy as the proxy in front, and it cuts BOTH
  ways.** `X-Forwarded-For` is client-settable when nothing strips it.
  - *Under*-limiting: a direct-to-internet deployment lets an attacker mint a
    fresh bucket per request by varying the header, so it degrades to no
    limiting for that attacker.
  - ⚠ *Over*-limiting, which an earlier draft of this ADR wrongly said could not
    happen: because the key is taken **verbatim** from the header's first entry
    and the limiter runs **before auth**, an unauthenticated client can drain a
    *named victim's* bucket by sending `X-Forwarded-For: <victim-ip>`. **"Run
    behind a proxy" does not fix this on the most common configuration** —
    nginx's documented `proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for`
    **appends**, producing `client-supplied, real-ip`, and the oracle's
    derivation takes the *first* entry, which is the attacker's. Only a proxy
    that **overwrites** the header (`proxy_set_header X-Forwarded-For $remote_addr`)
    makes the key trustworthy.

  This is the oracle's own key derivation, kept for parity. Deployments that
  cannot guarantee an overwriting proxy should set `AGNOSAI_RATE_LIMIT=0` and
  limit upstream. Revisiting the derivation itself — a trusted-proxy count, or
  preferring `X-Real-IP` — is a change to `extract_client_key` and therefore a
  separate decision with its own ADR.
- **Memory is bounded by three mechanisms, because the obvious one is not
  enough.** ⚠ An earlier version of this ADR claimed cardinality was "bounded in
  time" by idle eviction. **That was false**: `agnosai_rate_limit_evict_stale`
  had exactly one caller in the whole tree and it was a test, so nothing swept
  at all — and even once it did, idle eviction alone cannot bound a client that
  presents a *fresh* key every request, because every bucket stays well inside
  the idle window. What actually bounds it:
  1. **The key is capped at `AGNOSAI_RL_MAX_KEY_BYTES` (64)** — enough for a
     textual IPv6 address with a zone id. Without it a single request could hand
     majra a megabyte-long key, of which majra takes a *permanent* copy, and
     anything over 4 KiB takes the freelist's large path and mmaps a VMA that is
     never unmapped — an unauthenticated route to exhausting `vm.max_map_count`.
  2. **The sweep runs**, amortised one walk per `AGNOSAI_RL_SWEEP_EVERY` (256)
     checks, at the oracle's 300 s idle threshold.
  3. **Above `AGNOSAI_RL_MAX_KEYS` (4096) the sweep escalates** to a 1 s
     threshold and then to 0. ⚠ The zero rung discards live clients' consumed
     tokens, letting them briefly burst again; that is the deliberate trade,
     since the alternative is unbounded attacker-controlled growth and a flood
     of fresh keys is already outside what per-key limiting can meter.
- **The three unauthenticated probes are exempt.** `/health`, `/ready` and
  `/metrics` — exactly the routes `agnosai_route_needs_auth` leaves open — are
  never refused. ⚠ Without this the default is a self-inflicted outage: a kubelet
  sends neither proxy header, so it keys as the shared `"unknown"` bucket, and an
  unauthenticated flood of `GET /health` drains that bucket until the liveness
  probe takes 429 and the orchestrator kills the pod — a crash loop manufactured
  by the control meant to prevent one. The exemption costs nothing an attacker
  wants: the probes read no state and touch no crypto.
- If the oracle ever installs its own middleware, revisit against whatever rate
  Rust picks and narrow the divergence to whatever remains.
