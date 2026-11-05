# sigil: a burst of more than 64 concurrent ECDSA verifies fails the overflow instead of waiting

**Found by:** agnosai (2026-11-05, 2.2.3 audit). **Version:** sigil 3.13.15 (the cyrius 6.7.18 fold).

## Summary

Since 3.13.14 (CYRIUS-2026-0040) lanes come from a pool of `SIGIL_CRYPTO_BANKS` = 64 and are never shared,
which closes the aliasing bug. A scoped entry point that finds every lane held **refuses**:
`ecdsa_p256_verify` returns 0. The returned value does not distinguish "no lane free" from "invalid
signature". So a caller doing a TLS handshake reads a pool-full refusal as a bad certificate signature, and
fails the handshake.

## Repro

100 threads, released together by a spin barrier, each calling
`ecdsa_p256_verify_der(pk, "lane-probe", 10, sig, 72)` once on a **valid** P-256 signature (made with
`openssl dgst -sha256 -sign`, and checked with `openssl dgst -verify`).

## Observed

Five runs: **64 accepted and 36 refused every time**, and `crypto_banks_claimed()` reads 0 after the joins,
so no lane leaked. With 300 verifies per thread, about 36% of all verifies are refused.

## Expected / ask

A bounded wait for a free lane, a few milliseconds at most, before refusing, or a distinct return for "no
lane free" so a caller can retry. Either keeps the never-share guarantee.

## Impact on agnosai

agnosai's outbound native TLS (hoosh when it is https, the remote registry, the AGNOS and security-audit
tools, OTLP export) verifies server certificates with these entry points. It runs 100 pool workers plus a
thread per crew. A burst of more than 64 simultaneous handshakes against ECDSA certificates fails the
overflow. agnosai could cap concurrent outbound TLS at 64 with a counting semaphore, which the stdlib does
not have (its roadmap B6).
