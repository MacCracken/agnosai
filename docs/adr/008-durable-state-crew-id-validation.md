# 008 — `durable_state` validates `crew_id`

## Status: Accepted

## Context

`rust-old/src/orchestrator/durable_state.rs` builds its on-disk path with

```rust
fn path_for(&self, crew_id: &str) -> PathBuf {
    self.base_dir.join(format!("{crew_id}.json"))
}
```

and validates `crew_id` **nowhere** — not in `FileStateStore::new`, not in
`path_for`, not in `save`, not in `load`. Two consequences follow, both verified
rather than assumed:

1. **`Path::join` with an absolute argument discards the base entirely.** A
   `crew_id` of `/etc/cron.d/pwn` yields `/etc/cron.d/pwn.json`, not
   `{base_dir}/etc/cron.d/pwn.json`. Cyrius string concatenation does not behave
   that way — it produces `{base_dir}//etc/cron.d/pwn.json` — so even a literal
   port diverges here whether or not anyone intends it.
2. **`..` traverses on both**, because the kernel resolves it. `../../etc/passwd`
   escapes `base_dir` in Rust and in Cyrius alike.

So `save` is an arbitrary write wherever the parent directory already exists,
and `load` is an arbitrary read of any path ending `.json`.

Inside `rust-old` this is unreachable: `grep` for `StateStore`,
`serialize_crew_state`, `deserialize_crew_state` and `durable_state` across the
whole tree returns exactly one line, `pub mod durable_state;`. There are zero
non-test callers, so no code path in the Rust binary ever supplies a `crew_id`
to it.

That is precisely why it matters. A module with no internal callers exists for
its **consumers** — Agnostic, daimon, joshua, kiran — and a consumer that
resumes a crew from an id that arrived over the wire hands the id straight
through. The oracle is not safe-by-construction; it is safe-by-having-no-callers.

## Decision

`agnosai_state_store_save` and `agnosai_state_store_load` reject an unsafe
`crew_id` before touching the filesystem, via a public
`agnosai_crew_id_is_safe`. Rejected: an empty id, an embedded NUL, `/`, `\`, and
any id beginning with `.` (which covers `.`, `..`, and dotfiles in one test).

Rejection returns `agnosai_error_io` with the detail `unsafe crew id: {id}`.
There is no oracle error to match, because the oracle cannot reject; `Io` is the
right kind because the operation that failed is the filesystem access, and it
keeps `save`'s contract to a single error type.

The check is exposed publicly rather than kept private so a consumer can
pre-validate an id at its own trust boundary instead of discovering the problem
as a failed save.

## Consequences

**Every id the oracle's own tests use still passes**: `crew-1`, `crew-x`,
`nonexistent-crew`, `my-crew-123`, and uuid strings. The oracle's test suite
ports without modification, which is the evidence that this tightening costs
nothing a real caller was doing.

**A hierarchical id stops working.** A consumer wanting `tenant-a/crew-1` to nest
into a subdirectory must now flatten it (`tenant-a__crew-1`) or hold one store
per tenant. This is the only behaviour genuinely lost, and it was never a
documented feature — it was a side effect of unvalidated concatenation.

**The absolute-path divergence is closed rather than documented.** Because
separators are refused, the `Path::join`-replaces-base behaviour can no longer
be reached, so there is no residual difference between the two implementations
for any accepted input. Without this decision the port would have had to
document a permanent behavioural difference it could not fix.

**`base_dir` is still unvalidated**, matching the oracle. It comes from the
application's own configuration, not from a request, and `FileStateStore::new`
takes it verbatim.

This follows the precedent of [ADR 007](007-audit-redirect-revalidation.md),
which closed an SSRF-via-redirect bypass the oracle has. Same shape: a public
surface where the oracle's safety depended on nobody calling it, tightened with
the reasoning written down rather than left as a silent divergence.
