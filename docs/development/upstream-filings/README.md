# Upstream filings — drafts for the user to send

Drafts of issues agnosai has found in its dependencies, written here so the evidence travels with them.
agnosai does not write into another repo. The user files each one (in the dependency's
`docs/development/issues/`, or wherever that project takes reports) and then deletes the draft here, or
marks it filed with the upstream path.

Each draft names the version it was measured against. Re-check it before sending if that version has moved
(roadmap B16's rule).

| Draft | Dependency | Severity for agnosai |
|---|---|---|
| [2026-11-05-kavach-wasm-capture-stdin-deadlock.md](2026-11-05-kavach-wasm-capture-stdin-deadlock.md) | kavach 3.14.2 | ⚠ High: a WASM guest can hang a worker for good, past kavach's own deadline |
| [2026-11-05-sigil-lane-pool-refuses-a-burst.md](2026-11-05-sigil-lane-pool-refuses-a-burst.md) | sigil 3.13.15 | Medium: a burst of more than 64 concurrent ECDSA verifies fails the overflow |
| [2026-11-05-kavach-wasm-store-limits.md](2026-11-05-kavach-wasm-store-limits.md) | kavach 3.14.2 | Low: no instance, table or memory-count caps for a WASM guest |
| [2026-11-05-sandhi-chunked-request-bodies.md](2026-11-05-sandhi-chunked-request-bodies.md) | sandhi 1.13.7 | Low: a chunked request body is 501 |
| [2026-11-05-sigil-rsa-over-4096.md](2026-11-05-sigil-rsa-over-4096.md) | sigil 3.13.15 | Low: an RSA key over 4096 bits cannot verify |
