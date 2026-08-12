# 020 — `output_filter` is wired into task output; the oracle never wires it

## Status: Accepted (2026-08-12)

## Context

`src/server/output_filter.cyr` is a faithful port of
`rust-old/src/server/output_filter.rs` — 20 functions that scan model output for
leaked system prompts, API keys, emails, phone numbers and SSNs, and redact the
first three. It carries its own passing assertions.

**It had no caller, and neither does the oracle's.** The only mention of
`output_filter` in the entire Rust tree is:

```
rust-old/src/server/mod.rs:11:pub mod output_filter;
```

That is the module declaration and nothing else — no handler, no runner, no test
outside its own `#[cfg(test)]` block calls `scan` or `redact`. So the port was
**at parity**: dead upstream, dead here.

The asymmetry is what made this worth changing. `prompt_guard` — the *inbound*
half of the same defence — **is** wired in the oracle
(`rust-old/src/orchestrator/crew_runner.rs:651,660,669,671,757`), and the port
wires it at four matching sites in `src/orchestrator/crew_runner.cyr`. So agnosai
sanitised everything going *into* a model and inspected nothing coming *out*,
while shipping a complete, tested implementation of the outbound half.

A 2026-08-12 P(-1) sweep surfaced this as "built, tested, never called". That
framing was half right: the fact was correct, the implication that it was a port
defect was not.

## Decision

**Wire `agnosai_output_scan` into `agnosai_execute_task`, on the model response,
before it becomes a `TaskResult`.** This diverges from the oracle deliberately.

Two behaviours, split on whether they destroy data:

1. **Scanning is unconditional on model output.** Every finding is logged at
   `SK_WARN` with the task id, category and pattern. It changes no output.

   ⚠ **It is NOT applied unconditionally to the no-LLM placeholder path**, and
   that split is driven by measurement. `agnosai_output_scan` costs **16.7 µs**
   (the system-prompt build it needs is a further 2.6 µs) against a crew path of
   roughly 40 µs per task. Scanning the placeholder unconditionally measured
   **+90%** on `run_crew_10_tasks_sequential` (394 → 749 µs) and **+43%** on
   `run_crew_1_task_sequential`. The placeholder echoes the task description —
   request input that `prompt_guard` has already sanitised — so there is no model
   output there to protect, and paying 90% for it is not a trade worth making.
   The LLM path is network-dominated, where the same 16.7 µs is noise.

   The placeholder path is therefore filtered only when redaction is enabled,
   which is also what makes the wiring drivable from a suite.

2. **Redaction is OFF by default**, behind `AGNOSAI_OUTPUT_REDACT=1`.
   `agnosai_output_redact` rewrites the response, and a task whose legitimate job
   is to return an email address, a phone number, or a key-shaped token would
   have its answer mangled with no way for the caller to tell. Silently
   corrupting correct output is a worse failure than logging a warning about it.

The system prompt handed to `agnosai_contains_prompt_leak` is the same one the
request was built from — `agnosai_wrap_system_prompt(_agnosai_crew_build_system_prompt(agent))`
— so a leak is distinguished from ordinary text rather than guessed at.

## Consequences

- **A divergence from the parity oracle, recorded here** as CLAUDE.md requires.
  It joins [ADR 007](007-audit-redirect-revalidation.md) (SSRF revalidated on
  every redirect hop), [ADR 009](009-auth-constant-time-secret-compare.md) and
  [ADR 010](010-jwt-require-configured-iss-aud.md) — all cases where the port is
  deliberately stricter than the Rust it came from.
- **Default behaviour is observably unchanged** except for new `WARN` lines. No
  wire bytes move unless an operator sets `AGNOSAI_OUTPUT_REDACT`.
- **Measured cost on the crew path: +3.4% at one task, +1.0% at ten**
  (`run_crew_1_task_sequential` 94.5 → 97.8 µs, `run_crew_10_tasks_sequential`
  394.4 → 398.4 µs, `run_crew_10_tasks_parallel_4` flat). That residual is mostly
  the crew-timeout deadline check landing in the same release, not the filter,
  which is free on this path with redaction off.
- ⚠ **The LLM arm of the wiring is not mutation-covered.** Reverting
  `agnosai_execute_task` to use the raw response passes the whole suite, because
  that arm needs a live gateway — the same limitation `_agnosai_otlp_post`
  carries. The placeholder arm IS covered: unwiring it fails two assertions.
  Read the call site; do not trust the suite alone for the LLM half.
- ⚠ **A redaction-enabled deployment can corrupt legitimate output.** That is the
  explicit trade the flag exists to make the operator own. If a crew's purpose is
  extracting contact details, redaction must stay off for that deployment.
- ⚠ **The scan is not a security boundary.** It is pattern matching over model
  output: it will miss novel key formats and non-US phone/SSN shapes, and it can
  false-positive on ordinary prose. It reduces accidental leakage; it does not
  prevent a determined exfiltration.
- If the oracle ever wires its own `output_filter`, this ADR should be revisited
  against whatever call site Rust chooses, and the divergence narrowed to
  whatever remains.
