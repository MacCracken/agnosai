# 006 — cx bytecode + kavach for sandboxed tool execution (replacing WASM)

## Status

Accepted — 2026-07-28. Supersedes the WASM approach in `rust-old/src/sandbox/wasm.rs`,
`rust-old/src/tools/wasm_tool.rs`, `rust-old/src/tools/wasm_loader.rs` (955 lines).
Resolves port-plan blocker #6.

## Context

The Rust tree sandboxes untrusted tool code with wasmtime. `WasmSandbox::execute`
(`wasm.rs:124`) gives a deliberately narrow contract: **stdin piped in, stdout captured,
exit code out, nothing else** — plus four enforcement mechanisms, all in-process:

| Mechanism | wasmtime |
|---|---|
| Memory bound | `StoreLimitsBuilder::memory_size`, `trap_on_grow_failure` |
| CPU bound | fuel (`store.set_fuel`) |
| Wall-clock bound | epoch deadline + background ticker thread |
| Syscall/ambient authority | WASI, explicitly built with **only** stdin + stdout |

Cyrius has no embeddable WASM engine, and **WASM is an explicit non-goal** for the
toolchain — its roadmap states "CYX register-bytecode ≠ WASM". An earlier revision of the
port plan concluded from this that the capability could not ship. That conflated the
*format* with the *capability*: what these 955 lines buy is sandboxed portable execution of
untrusted tool code, and the format is incidental.

The cyrius **cx** arc exists precisely because a consumer "hit the wasm-shaped wall". It
ships today, installed:

- `cycc_cx` (in `cross_bins`) compiles `.cyr` → `.cyx` — `"CYX\0"` magic + entry point,
  fixed 4-byte instructions, register-based.
- `cxvm` (in `[release].bins`) reads a `.cyx` on **stdin** and executes it; the guest's exit
  code passes through.

Verified on this box before this ADR was written: a 6×7 program compiled and returned
exit 42 through `cat prog.cyx | cxvm`.

## Decision

**Port the tool sandbox onto cx, with kavach providing isolation.** The successor to
`wasm_tool.rs` compiles tool source to `.cyx` via `cycc_cx` and executes it by spawning
`cxvm` inside a kavach sandbox.

### The security boundary is kavach, not cxvm

This is the part that must not be misread, and it is the single biggest difference from the
wasmtime design.

**`cxvm` is not a sandbox.** Its opcode `0x70` (`syscall`) dispatches the guest's syscall
straight to the host kernel (`programs/cxvm.cyr:309-311`). It translates pointer arguments
from guest-virtual to real addresses and otherwise passes everything through, including a
generic 6-argument fallthrough for any syscall it does not special-case. There is **no
allowlist, no filtering, and no capability model** — grepping the VM for
`seccomp|allowlist|deny|forbid` returns zero hits.

Running an untrusted `.cyx` under a bare `cxvm` is therefore equivalent to running
untrusted native code. Every isolation guarantee `WasmSandbox` provided must come from
kavach instead:

| Mechanism | wasmtime (was) | cx + kavach (is) |
|---|---|---|
| Ambient authority | WASI ctx with only stdin/stdout | **kavach seccomp** (`strict`/`basic` profiles) + **landlock** rules |
| Filesystem | none linked | **landlock** rules, default-deny |
| Memory bound | `StoreLimits` | cxvm's fixed **64 KB data segment** + kavach rlimits |
| CPU / wall-clock | fuel + epoch deadline | **kavach process timeout** — there is no fuel equivalent |
| Result channel | stdout pipe, exit code | same — process stdout + exit code |

Net: filesystem and network confinement get *stronger* (kernel-enforced seccomp/landlock
rather than in-process linking choices). Fine-grained CPU metering gets *weaker* — fuel
gave deterministic instruction budgets; a process timeout does not. Tools that need to be
billed or bounded by instruction count lose that.

**Hard requirement: no code path may execute a `.cyx` outside a kavach sandbox.** A
`cxvm` spawn that is not wrapped is a full sandbox escape, not a degraded one.

### Input channel

WASM took tool input on stdin. cxvm takes the *bytecode* on stdin, so that channel is
consumed. Tool input therefore arrives by argv/env into the guest, or via a file opened
under a landlock rule — to be settled when M7 lands, but it is a real interface change from
the Rust design and cannot be a drop-in.

## Consequences

**Gained**
- The capability ships in v2.0.0 rather than being cut. `wasm_tool.rs` and `wasm_loader.rs`
  get successors instead of being deferred indefinitely.
- One toolchain: tools are written in Cyrius, compiled by the same compiler, no second
  language runtime to vendor or track for CVEs.
- Kernel-enforced confinement rather than in-process.

**Lost / accepted risk**
- **No fuel.** Wall-clock timeout only. A tool can burn its whole budget spinning.
- **Not a drop-in for existing `.wasm` tools.** Anything shipped as WASM must be rewritten
  in Cyrius. `examples/wasm-tools/` and the tool SDK do not port.
- **Isolation is entirely kavach's.** A kavach misconfiguration is a full escape. This
  deserves a threat-model entry and a test that asserts a `.cyx` attempting `open("/etc/passwd")`
  is refused.

**Constrained by cx arc A** — all verified, not assumed:
- `cxvm` is **x86-Linux only** (raw syscalls, no ESYSXLAT). Cross-OS is arc C.
- **64 KB code/data caps** (1 MB `.cyx` file ceiling). Cap lifting is arc C.
- **Float literals silently miscompile** — `EMIT_FLOAT_LIT` emits raw rational-pair bits,
  flagged in the cyrius roadmap, fixed in arc B. **Tool code must be integer-only until
  then**, and the loader should reject float literals rather than trust them.
- cx is a *bare* target: no stdlib is auto-injected, and an un-included function links to an
  undefined-fn trap (SIGILL, exit 136) rather than a link error.

**Revisit at cx arc B** (float) **and arc C** (cross-OS, caps). Until arc C, tool execution
is a Linux-x86-only feature and the other platforms must degrade explicitly rather than
silently.
