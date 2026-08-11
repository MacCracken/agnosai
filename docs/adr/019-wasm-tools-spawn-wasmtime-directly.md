# 019 — WASM tools go through kavach's backend (superseded transport, resolved)

## Status: Accepted — supersedes the transport half of [ADR 006](006-cx-tool-sandbox.md)

> ### Revision history, because this ADR changed its own answer inside a day
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

- `src/sandbox/wasm.cyr` builds a `SandboxConfig` with `Backend.WASM`, the policy and the
  timeout, calls `config_stdin` with the marshalled `{"parameters": …}`, and dispatches
  through the existing `src/sandbox/kavach_bridge.cyr`.
- **kavach owns the confinement**, as it does for cx and for the process backend. That is
  the whole reason to prefer this over a direct spawn: seccomp and landlock wrap the
  `wasmtime` process itself, on top of wasmtime's own WASI sandbox around the guest.
- **`agnosai_wasm_available()` reports `backend_is_available(Backend.WASM)`**, so a host
  without `wasmtime` gets a clean refusal rather than a spawn failure.

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

## Consequences

**Gained**
- The stdin contract is preserved exactly, so an existing `.wasm` tool built against the
  published SDK runs unmodified.
- Real exit codes and real stderr, so the oracle's failure taxonomy survives.
- Fuel, memory and preopen bounds enforced by kavach, plus seccomp and landlock around
  the runtime process.
- One confinement story across cx, process and WASM.

**Lost / accepted risk**
- **`wasmtime` is a host requirement** and is not installed on the current development
  box. The execute path degrades to a clean refusal naming the missing binary — an
  assertable arm, not a silent skip — and the loader, validator and output ladder are all
  testable without it. Record it alongside `python3` and `cxvm` in the state doc's gates.
- **The `-1` / `-2` / `-3` exit taxonomy the in-process oracle produced cannot be
  reproduced through a CLI.** wasmtime's process exit code is what is available; telling
  "trapped" from "epoch deadline" from "out of fuel" has to come from parsing stderr,
  which is best-effort. Covered by tests that feed the classifier captured stderr as data
  rather than by running a real trap.
- **kavach 3.11.8 is a floor**, not a preference: `[deps.kavach]` must not go below it,
  because 3.11.7 silently reports every WASM failure as a success. Pinned in
  `cyrius.cyml` and asserted by `tests/sandbox_wasm.tcyr`.

**`src/sandbox/mod.cyr`'s exclusion of `wasm.rs` is retired.** Its header said WASM was
"excluded rather than postponed … an explicit cyrius non-goal", which ADR 006's own
correction already overturned; the full-port mandate settles it. 521 lines and 11 oracle
tests are owed under M11.
