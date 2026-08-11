# 019 — WASM tools go through kavach's backend (superseded transport, resolved)

## Status: Accepted — validated end to end 2026-08-11; supersedes the transport half of [ADR 006](006-cx-tool-sandbox.md)

> ⚠ **The filename is wrong and is kept for its URL.** `019-wasm-tools-spawn-wasmtime-directly.md`
> names the *drafted* decision, which was reversed before it shipped. The decision of
> record is "go through kavach's WASM backend". Renaming it would break the six links
> that name the path — `src/tools/mod.cyr`, `src/sandbox/wasm.cyr`,
> `src/sandbox/mod.cyr`, `docs/adr/006-cx-tool-sandbox.md`,
> `docs/development/roadmap.md` and `CHANGELOG.md`.

> ### Revision history, because this ADR changed its own answer inside a day — and then had to prove it
>
> **Drafted 2026-08-09** deciding to spawn `wasmtime` **directly**, because kavach's
> WASM backend could not carry a tool protocol: it was hardcoded unavailable, had no
> stdin channel, and discarded the guest's exit code.
>
> **Revised 2026-08-09, same day.** All three were filed upstream and fixed in
> **kavach 3.11.8**, and agnosai pins kavach as a git dep (`[deps.kavach]`), not through
> the cyrius stdlib fold — so the fix arrived by bumping one tag, not by waiting for a
> toolchain release. The direct-spawn decision never shipped a line of code.
>
> **The decision below is the revised one: go through kavach.** The draft's transport is
> recorded in "Rejected alternative" so the reasoning is not lost.
>
> **Validated 2026-08-11**, and only then. `wasmtime` 47.0.3 was installed on the
> development box, `tests/sandbox_wasm.tcyr` grew `_t_execute_real_module`, and a real
> 36-byte guest went in and came back out. That first run found **two** bugs that had
> made the path unrunnable since the day it was written — see *What validation found*
> below. Until 2026-08-11 this ADR described a decision that had never once executed.

ADR 006's decision stands: the sandboxed-tool capability ships, and its 2026-08-07
correction is right that `tools/wasm_tool.rs` and `tools/wasm_loader.rs` are **in scope**
rather than replaced by cx. This ADR settles *how* a `.wasm` module is executed.

## Context

ADR 006's correction named kavach's wasmtime backend as the target, because it passes
`--fuel`, `--max-memory-size` and `--dir` preopens — the deterministic CPU bound, the
memory bound and the filesystem bound that the cx path cannot offer.

Verified against kavach **3.11.7**, that backend could not host a tool:

1. **Unreachable.** `backend_is_available` hardcoded `Backend.WASM → 0` while
   `wasm_health` beside it did the correct probe. `sandbox_create` refuses an unavailable
   backend, so `wasm_exec` could not be entered through the public API *even with
   `wasmtime` installed*.
2. **No stdin.** `wasm_exec` went through `exec_capture`, which `dup2`s stdout only; the
   child inherited the parent's fd 0. The oracle's entire contract is JSON on stdin
   (`rust-old/src/sandbox/wasm.rs:128`), and the published tool SDK pins that wire
   format. A WASM tool that cannot be given parameters is not a tool.
3. **No exit code.** `backend_capture_finish` stamped 0 on any successful capture, so a
   trapping module reported success — while the oracle branches on `exit_code != 0` for
   every failure mode it has.

All three are fixed in **kavach 3.11.8**
(`kavach/docs/development/issues/2026-08-09-wasm-backend-unreachable-no-stdin-no-exit-code.md`):
`backend_is_available` probes with `_wasm_binary_path`; `confine_capture_input` adds the
stdin pipe and `SandboxConfig.stdin` carries the payload; and `wasm_exec` moved off
`exec_capture` onto the confined capture, which also restored the guest's **stderr** and
the `timeout_ms`/`policy` that path had been ignoring.

## Decision

**Execute `.wasm` modules through kavach's WASM backend**, with the tool's JSON input set
via `config_stdin` and the result read from `ExecResult`'s exit code, stdout and stderr.

- `src/sandbox/wasm.cyr`'s `agnosai_wasm_execute` builds a `SandboxConfig` with
  `Backend.WASM` and the sandbox's `timeout_ms`, calls `config_stdin` with the marshalled
  `{"parameters": …}`, and drives kavach's create → RUNNING → exec → STOPPED → DESTROYED
  lifecycle.
  - ⚠ It **mirrors** `agnosai_kavach_execute` (`src/sandbox/kavach_bridge.cyr`) rather
    than calling it, because that entry point maps an agnosai isolation level onto a
    backend and this must pin `Backend.WASM` and set `config_stdin`. Two copies of the
    lifecycle; CLAUDE.md says extract at the third. An earlier draft of this ADR said it
    "dispatches through" the bridge — it does not, and the difference is load-bearing:
    it is exactly what caused bug 2 below.
  - ⚠ It sets **no policy**, so kavach's `config_new()` default — `policy_basic()`,
    seccomp on, everything else zero — is what applies. See *Residual* below for what
    that costs.
- **kavach owns the confinement**, as it does for cx and for the process backend. That is
  the whole reason to prefer this over a direct spawn: seccomp and landlock wrap the
  `wasmtime` process itself, on top of wasmtime's own WASI sandbox around the guest.
- **`agnosai_wasm_available()` reports `backend_is_available(Backend.WASM)`**, so a host
  without `wasmtime` gets a clean refusal rather than a spawn failure.

## What validation found — 2026-08-11

Two bugs, one on each side of the seam. Neither was reachable by any test that existed
before a real module ran, which is the point worth carrying forward: **the guarded arms
of `tests/sandbox_wasm.tcyr` all stopped at the header check, the path resolution or the
error text**, and every one of them passes against a backend that can never run anything.

1. **kavach emitted argv `wasmtime` rejects outright.** `_wasm_append_limits` wrote
   `--max-memory-size <bytes>` and `--fuel <N>` as top-level `wasmtime run` flags. Both
   live in wasmtime's `-W` option group, so wasmtime 47 answers
   `error: unexpected argument '--max-memory-size' found` and runs nothing. ⚠ **This was
   never a version skew** — `-W` has existed since wasmtime 14 (2023) and the top-level
   spelling was never valid. It survived because kavach's own two WASM tests both fail
   before argv is assembled, and the backend was unreachable through the public API until
   3.11.8. Fixed in **kavach 3.11.10**, which emits the `-W key=value` pair form.

2. **agnosai never called `kavach_init`, so the dispatch table was empty.** Because
   `agnosai_wasm_execute` reaches `sandbox_create`/`sandbox_exec` directly rather than
   through `agnosai_kavach_execute` — the only other caller of
   `_agnosai_kavach_ensure_init` — every execute answered `backend not available: wasm`.
   ⚠ **`agnosai_wasm_available()` did not catch it, and that is why it went unnoticed**:
   `backend_is_available` probes the *filesystem* for the wasmtime binary while
   `backend_dispatch_exec` looks for a *registered function pointer*. They answer
   different questions, so availability reported 1 and the very next call reported the
   backend missing. Fixed by calling `_agnosai_kavach_ensure_init()` first thing in
   `agnosai_wasm_execute`; `_t_execute_real_module` is the guard.

The two compound: with no `wasmtime` on the box, bug 2's guard returned early and nothing
downstream ever ran, so installing the runtime is what exposed both at once.

## Rejected alternative — spawning `wasmtime` directly

Drafted first, and correct while kavach 3.11.7 was the only option: build the argv here
and run it through `agnosai_spawn_capture_input` (`src/sandbox/spawn.cyr`), the primitive
`sandbox/cx.cyr` and `sandbox/python.cyr` use.

Rejected once 3.11.8 landed, for one reason that outweighs the rest: **it does not
inherit kavach's seccomp/landlock confinement.** wasmtime's WASI sandbox would be the
only boundary — which is what the oracle relied on, so it was parity rather than a
regression, but it is strictly weaker than routing through kavach and it would have left
the two sandbox paths in this tree confined differently for no reason a reader could
infer.

It would also have duplicated `--fuel` / `--max-memory-size` / `--dir` argv construction
that kavach already has, and left agnosai owning a second copy of it.

⚠ **Validation gave that second argument a twist worth recording.** kavach's copy of the
argv was *wrong* — bug 1 above — and had been since it was written, so "kavach already has
it" was not the same as "kavach already has it right". The argument still holds, and more
strongly than it read at the time: because there is one copy and it lives upstream, fixing
it fixed every kavach consumer at once, in kavach 3.11.10. Had agnosai built its own argv
here, the fix would have been local and the broken copy would still be shipping to
everyone else. Nothing about this says a direct spawn would have got the spelling right —
it might have made the same mistake — only that centralising the mistake is what made it
findable and fixable in one place.

## Consequences

**Gained**
- The stdin contract is preserved exactly, so an existing `.wasm` tool built against the
  published SDK runs unmodified. Confirmed by a real guest run, not by inspection.
- Real exit codes and real stderr, so the oracle's failure taxonomy survives.
- seccomp and landlock around the `wasmtime` process, on top of wasmtime's own WASI
  sandbox around the guest.
- One confinement story across cx, process and WASM.

**Lost / accepted risk**
- **`wasmtime` is a host requirement.** Absent, `agnosai_wasm_available()` answers 0 and
  the execute path degrades to a clean `Sandbox` error naming the missing binary — an
  assertable arm, not a silent skip. It belongs alongside `python3` and `cxvm` in the
  state doc's gates. ⚠ **This is no longer the arm the development box takes**: wasmtime
  47.0.3 was installed 2026-08-11 and `tests/sandbox_wasm.tcyr` now exercises both arms,
  with a loud guard so a regression in detection cannot quietly skip the real one.
- **The `-1` / `-2` / `-3` exit taxonomy the in-process oracle produced cannot be
  reproduced through a CLI.** wasmtime's process exit code is what is available; telling
  "trapped" from "epoch deadline" from "out of fuel" has to come from parsing stderr,
  which is best-effort. Covered by tests that feed the classifier captured stderr as data
  rather than by running a real trap.
- **kavach 3.11.10 is the floor**, raised from 3.11.8 by validation: 3.11.7 silently
  reports every WASM failure as a success, and 3.11.8/3.11.9 emit argv `wasmtime` refuses,
  so the backend cannot run at all below 3.11.10. `[deps.kavach]` pins 3.11.10 in
  `cyrius.cyml`, and `_t_execute_real_module` is what fails if it ever goes below.

**Residual — the memory and fuel bounds are not the oracle's.** Recorded rather than
chased, because it is a code question and not a decision:

- `agnosai_wasm_sandbox_new` stores `max_memory_bytes = 64 MiB` and `fuel = 1e9` (the
  oracle's constants) and exposes both through accessors, but **neither is passed to
  kavach**. `agnosai_wasm_execute` sets only `config_backend`, `config_timeout_ms` and
  `config_stdin`.
- kavach derives its own instead: memory from `SandboxPolicy_memory_limit_mb(policy)`,
  which is **0** under the default `policy_basic()` — so no `-W max-memory-size` is
  emitted at all — and fuel from `_wasm_fuel_from_timeout(timeout_ms)`, which is
  `timeout_ms × 1e6`, i.e. **3e10** at the default 30 s timeout rather than the oracle's
  1e9.
- The wall clock and ambient-authority bounds are intact: `timeout_ms` is passed, and no
  `--dir` preopen is emitted because agnosai never sets a workdir, which matches the
  oracle's WASI context of stdio only.

**`src/sandbox/mod.cyr`'s exclusion of `wasm.rs` is retired, and the work is done.** Its
header said WASM was "excluded rather than postponed … an explicit cyrius non-goal", which
ADR 006's own correction already overturned; the full-port mandate settled the rest. The
oracle's 521 lines are ported as `src/sandbox/wasm.cyr` (337 lines) and its 11 tests as
`tests/sandbox_wasm.tcyr`; **M11 closed 2026-08-10**. `mod.cyr`'s header still says this
is "owed under M11" — that sentence is stale, not this one.
