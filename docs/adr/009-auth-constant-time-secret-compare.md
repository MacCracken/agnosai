# 009 — Constant-time shared-secret comparison via SHA-256 digests

## Status: Accepted

## Context

`rust-old/src/server/auth.rs` compares a client's Bearer token against the
configured shared secret with a hand-rolled `constant_time_eq` (auth.rs:18-31):

```rust
let len_diff = a.len() ^ b.len();
let mut result: u8 = 0;
for i in 0..a.len().max(b.len()) {
    let x = if i < a.len() { a[i] } else { 0 };
    let y = if i < b.len() { b[i] } else { 0 };
    result |= x ^ y;
}
result == 0 && len_diff == 0
```

Its doc comment (auth.rs:16-17) states: *"Both length and content are compared in
constant time — no early return on length mismatch that would leak secret
length."*

**The content compare is constant time. The loop bound is not.** The loop runs
`max(a.len(), b.len())` times, and `a` is the attacker-supplied token. Sending a
1-byte token makes the loop run `secret.len()` times; sending progressively
longer tokens makes the runtime flat once the token exceeds the secret. The
secret's length is therefore recoverable by timing — precisely what the comment
promises it is not.

Secret *length* is much weaker than secret *content*, so this is a low-severity
flaw. But it is a flaw the oracle believes it does not have, which makes
transcribing it a poor default: the comment would have to be transcribed too,
and it would still be false.

Two candidate ports:

1. **Transcribe the oracle's loop.** Exact parity, keeps the leak, keeps a
   comment that misdescribes its own code.
2. **`ct_eq_bytes_lens` from `lib/ct.cyr:75`.** Rejected — it is worse. It
   returns 0 *immediately* on a length mismatch (`lib/ct.cyr:76`), so it leaks
   the same fact through a sharper, earlier signal.

## Decision

Compare **SHA-256 digests of both inputs** with `ct_eq_bytes` over a fixed
32 bytes:

```
sha256(str_data(a), str_len(a), &ha);
sha256(str_data(b), str_len(b), &hb);
return ct_eq_bytes(&ha, &hb, 32);
```

SHA-256 is fixed-output, so the comparison width is 32 bytes regardless of
either input's length, and `ct_eq_bytes` is constant time in the content. The
digest computation itself is linear in the *token* length, which the attacker
already knows because they chose it, and is independent of the secret's length.

## Consequences

**This is not a wire divergence.** The accept/reject set is byte-identical to the
oracle's: `sha256(a) == sha256(b)` iff `a == b` for any input an attacker can
construct, so every one of the oracle's five shared-secret assertions holds
unchanged, as do the near-miss cases (`tests/server_auth.tcyr::_t_secret_compare`
pins prefix, trailing-byte, trailing-space and case variants). Only the timing
profile differs, and only by removing a leak.

**Cost.** Two SHA-256 invocations per authenticated request instead of one byte
loop. On a path that already does TCP, TLS and JSON work this is not measurable;
`benches/orch.bcyr::auth_check_secret_ok` records the real figure so the claim is
not left as an assertion.

**What this does not fix.** The oracle's *comment* is wrong about the oracle's
*code*; that is upstream's to correct if the Rust tree is ever revived. Nothing
in agnosai depends on it.

**Precedent for the rest of M6.** Where the oracle's stated intent and its actual
behaviour disagree, and the behaviour is unobservable through the wire contract,
the port implements the stated intent and records the gap here. Where they
disagree and the behaviour *is* wire-observable — `iss`/`aud` absent passing, the
array-`aud` 401, jsonwebtoken's 60-second leeway — the port reproduces the
behaviour and the divergence is a separate, explicit decision. See
`docs/development/state.md`, "Four decisions waiting on the maintainer".
