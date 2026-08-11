# 006 — cx bytecode + kavach for sandboxed tool execution

## Status: Accepted in part — the "replacing WASM" half is retired

Accepted 2026-07-28, as **"cx + kavach instead of WASM"**. Two later passes cut that
in half, and the title above has been shortened to match:

- **Stands.** Sandboxed execution of Cyrius-authored tool bytecode ships, and **kavach —
  not the VM — is the security boundary**. `src/sandbox/cx.cyr` implements it, and
  `tests/sandbox_cx.tcyr::_t_cx_guest_cannot_open_etc_passwd` is the acceptance test this
  ADR asked for.
- **Retired.** This ADR did *not* supersede `rust-old/src/sandbox/wasm.rs`,
  `tools/wasm_tool.rs` or `tools/wasm_loader.rs`. Its own 2026-08-07 correction overturned
  that, and the full-port mandate settled it: all three are ported —
  `src/sandbox/wasm.cyr`, `src/tools/wasm_tool.cyr`, `src/tools/wasm_loader.cyr`.
- **Superseded on transport.** *How* a `.wasm` module runs is
  [ADR 019](019-wasm-tools-spawn-wasmtime-directly.md)'s: through kavach's WASM backend.

Resolves port-plan blocker #6.

> ### ⚠ Correction 2026-08-11 — the WASM path now runs, and this ADR's kavach citations are stale
>
> Re-verified on this box while reviewing the ADRs against the tree:
>
> - **`wasmtime` is installed** — 47.0.3 at `/usr/bin/wasmtime`. The 2026-08-07 correction
>   and ADR 019 were both written on a box that had none.
> - **kavach is a git dep pinned at 3.11.10** (`cyrius.cyml [deps.kavach]`), *not* the
>   6.5.10 stdlib fold the 2026-08-07 correction cites. Every line number in that
>   correction has moved: `_wasm_fuel_from_timeout` :9981, `_wasm_append_preopens` :9989,
>   `wasm_exec` :10072, `wasm_health` :10149, `wasm_destroy` :10154,
>   `backend_wasm_register` :10156 (registered at :11334).
> - **The flag spellings it quotes were never valid.** 3.11.10 emits
>   `-W max-memory-size=N` and `-W fuel=N` as `-W` option-group pairs
>   (`_wasm_push_w_opt`, `lib/kavach.cyr:10007`); `--max-memory-size` / `--fuel` as
>   top-level `wasmtime run` flags are a hard error on wasmtime 47. Only `--dir` preopens
>   are top-level, and 3.11.10 emits exactly one, for the config's `workdir`.
> - **The path executes end to end for the first time.** `agnosai_wasm_execute` reached
>   `sandbox_create` without ever calling `kavach_init`, so the backend dispatch table was
>   empty and every call answered "backend not available: wasm". It could not be observed
>   while `wasmtime` was absent, because the availability guard returned first.
>
> ⚠ **What "kavach provides the memory bound" turned out to mean.** See "Bounds that are
> available but not passed" under Consequences — the claim is true of kavach and not yet
> true of this tree's WASM call site.

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

Cyrius has no **embeddable** WASM engine, and WASM is an explicit non-goal for the
toolchain — its roadmap states "CYX register-bytecode ≠ WASM". An earlier revision of the
port plan concluded from this that the capability could not ship. That conflated the
*format* with the *capability*: what these 955 lines buy is sandboxed portable execution of
untrusted tool code.

⚠ It also conflated *embeddable* with *runnable*, which is the error the 2026-08-07
correction caught. kavach shells out to the `wasmtime` CLI, so a `.wasm` artifact runs on
this platform without anything being linked into the binary. See ADR 019.

The cyrius **cx** arc exists precisely because a consumer "hit the wasm-shaped wall". It
ships today, installed:

- `cycc_cx` compiles `.cyr` → `.cyx` — `CYX` magic + a format-version byte, fixed 4-byte
  instructions, register-based.
- `cxvm` reads a `.cyx` on **stdin** and executes it; the guest's exit code passes through.

Verified again 2026-08-11 (`/home/macro/.cyrius/bin/{cycc_cx,cxvm}`, cyrius 6.5.18): a 6×7
program compiles to 88 bytes and returns exit 42 through `cat prog.cyx | cxvm`.

⚠ **The magic is `CYX\x01`, not `CYX\0`.** The original text of this ADR said `"CYX\0"`;
the bytes a real `cycc_cx` emits are `43 59 58 01` — `CYX` followed by a **format version**.
A loader checking this ADR's spelling would reject every real `.cyx`, which is why
`src/sandbox/cx.cyr` checks `AGNOSAI_CX_VERSION` instead and says so in its header.

## Decision

**Sandboxed execution of Cyrius-authored tool bytecode rides cx, with kavach providing
isolation.** Tool source is compiled to `.cyx` via `cycc_cx` and executed by spawning
`cxvm` inside a kavach sandbox.

⚠ **As originally written this said "the successor to `wasm_tool.rs`". It is not one.**
`wasm_tool.rs` and `wasm_loader.rs` were ported directly onto kavach's WASM backend
(ADR 019). cx and WASM are **complementary backends**, exactly as the 2026-08-07 correction
put it: cx for tools written in Cyrius, WASM for everything already compiled to it.

⚠ **cx has no caller in `src/` today.** `src/sandbox/cx.cyr` is a complete, tested sandbox
primitive — compile, validate, run confined — but nothing in the tools layer constructs a
cx-backed tool; its only callers are `tests/sandbox_cx.tcyr`. Whether that is a gap or the
correct end state is **not recorded anywhere**, and this ADR will not invent a rationale for
it. What can be said is that no tool type was designed onto cx, and the input-channel
question below is unsettled as a direct consequence.

### The security boundary is kavach, not cxvm

This is the part that must not be misread, and it is the single biggest difference from the
wasmtime design.

**`cxvm` is not a sandbox.** Its `syscall` opcode dispatches the guest's syscall straight to
the host kernel. It translates pointer arguments from guest-virtual to real addresses and
otherwise passes everything through, including a generic 6-argument fallthrough for any
syscall it does not special-case. There is **no allowlist, no filtering, and no capability
model**.

Running an untrusted `.cyx` under a bare `cxvm` is therefore equivalent to running
untrusted native code. Every isolation guarantee `WasmSandbox` provided must come from
kavach instead:

| Mechanism | wasmtime (was) | cx + kavach (is) |
|---|---|---|
| Ambient authority | WASI ctx with only stdin/stdout | **kavach seccomp** (`basic` profile) + **landlock** rules |
| Filesystem | none linked | **landlock**, default-deny but for the interpreter's own directory |
| Network | none (WASI ctx has no sockets) | **network namespace**, demanded via `persistent_spawn_confined_ns(…, 1)` |
| Memory bound | `StoreLimits` | cxvm's fixed **64 KB data segment** |
| CPU / wall-clock | fuel + epoch deadline | wall-clock deadline enforced in `agnosai_cx_run` |
| Result channel | stdout pipe, exit code | same — process stdout + exit code |

Net: filesystem and network confinement get *stronger* (kernel-enforced seccomp/landlock/
netns rather than in-process linking choices). On the **cx** path, fine-grained CPU metering
is weaker — a process timeout is not a deterministic instruction budget.

Two refinements the implementation made that this ADR did not anticipate, both documented
at length in `src/sandbox/cx.cyr`'s header and worth knowing here:

- **The landlock ruleset is not `deny_all`.** A total-deny ruleset stops the child opening
  `cxvm` itself, so the guest exits 127 having never run — a refused *exec* that reads
  exactly like a refused *open*. The interpreter's own directory is allowed read-only and
  nothing else.
- **The deadline is enforced by agnosai, not kavach — for the PERSISTENT path.**
  `persistent_spawn_confined_ns(command, policy, require_ns)` (`lib/kavach.cyr:9043`)
  takes no timeout at all, so `agnosai_cx_run` owns the clock and
  `persistent_terminate`s on expiry. That is the load-bearing half and it holds.

  ⚠ An earlier pass of this review wrote "`SandboxConfig.timeout_ms` is ignored by every
  backend except WASM". **That is wrong** — exactly two backends read it
  (`grep SandboxConfig_timeout_ms lib/kavach.cyr` → two hits): the **process** backend at
  `:8875`, whose own comment reads *"The deadline the config carries — accepted since
  forever, enforced since 3.11.4"*, and the WASM backend at `:10100`. agnosai filed that
  process-backend gap itself and `docs/development/roadmap.md` records it as resolved in
  3.11.3–3.11.6. The correct statement is about the persistent API's signature, not about
  the config field.

> ### ⚠ Correction 2026-08-07 — "there is no fuel equivalent" is FALSE
>
> **kavach ships a wasmtime backend** — `wasm_exec` / `wasm_health` / `wasm_destroy`,
> registered by `backend_wasm_register()`, shelling out to the `wasmtime` CLI with a fuel
> bound, a memory bound and `--dir` preopens. (The version, line numbers and flag
> spellings originally cited here are corrected in the 2026-08-11 block at the top.)
>
> So the trade recorded here is real **only for Cyrius-authored `.cyx` tools**, which is
> what cx is for. It is not a property of the platform, and it was used to justify treating
> `tools/wasm_tool.rs` and `tools/wasm_loader.rs` as out of scope. **They are in scope** and
> port onto kavach's WASM backend. cx and WASM are complementary backends, not
> alternatives — this ADR's decision stands, its exclusion does not.

**Hard requirement: no code path may execute a `.cyx` outside a kavach sandbox.** A
`cxvm` spawn that is not wrapped is a full sandbox escape, not a degraded one.
`agnosai_cx_run` is the only runner and has no unconfined branch — no flag, no fallback, and
a failed `persistent_spawn_confined_ns` is a refusal rather than a reason to run the payload
anyway.

### Input channel — still unsettled

WASM took tool input on stdin. cxvm takes the *bytecode* on stdin, so that channel is
consumed: `agnosai_cx_run` writes the `.cyx`, closes fd 0 so the guest sees EOF, and the
guest's only channels back are stdout and its exit code.

This ADR said the alternative — argv/env, or a file opened under a landlock rule — would be
"settled when M7 lands". M7 landed and it was **not settled**: `agnosai_cx_run` takes
bytecode and a timeout, and has no input parameter. That is consistent with there being no
cx-backed tool type to need one, and it remains a real interface change from the Rust design
whenever one is built.

## Consequences

**Gained**
- The capability ships rather than being cut.
- One toolchain: tools are written in Cyrius, compiled by the same compiler, no second
  language runtime to vendor or track for CVEs.
- Kernel-enforced confinement rather than in-process.

**Lost / accepted risk**
- **No fuel *on the cx path*.** Wall-clock timeout only, so a `.cyx` tool can burn its
  whole budget spinning. This is a property of cx, **not** of the platform — kavach's WASM
  backend passes a fuel bound.
- **Isolation is entirely kavach's.** A kavach misconfiguration is a full escape. The test
  this ADR asked for exists — `tests/sandbox_cx.tcyr:528`, a `.cyx` calling
  `open("/etc/passwd")`, which gets **fd 3** under a bare `cxvm` and **EACCES** through
  `agnosai_cx_run`. ⚠ **The threat-model entry it also asked for does not exist**:
  `docs/development/threat-model.md` has no cx entry at all, and its data-flow table still
  routes tools through "Sandbox (wasmtime)".
- **Network isolation fails closed, and on some hosts that means tools do not run.** A
  network namespace needs a new user namespace when unprivileged, and hosts that restrict
  those get a spawn refusal rather than a guest with a network its policy denied.

**Bounds that are available but not passed** — ⚠ recorded 2026-08-11, and this is a
divergence from the oracle rather than a design choice anyone wrote down.
`agnosai_wasm_execute` builds its `SandboxConfig` with `config_new` / `config_backend` /
`config_timeout_ms` / `config_stdin` and never calls `config_policy`. It therefore inherits
`policy_basic()`, whose `memory_limit_mb` is **0**, so kavach emits no
`-W max-memory-size` at all and the oracle's 64 MiB `StoreLimitsBuilder::memory_size` bound
is not enforced. `agnosai_wasm_sandbox_max_memory_bytes` reports the configured 64 MiB
regardless. Fuel is passed, but as kavach's `timeout_ms × 1e6` derivation rather than
agnosai's `AGNOSAI_WASM_DEFAULT_FUEL`. Left as an escalation rather than fixed inside an
ADR pass; it is a code question, not a decision question.

**Constrained by cx arc A** — the float and size constraints re-verified 2026-08-11 against
cyrius 6.5.18; the platform constraint is carried forward from the original pass and was
**not** re-checked, since testing it needs a non-x86 host:
- `cxvm` is **x86-Linux only** (raw syscalls, no ESYSXLAT). Cross-OS is arc C.
- **64 KB code/data caps**, and a 1 MB `.cyx` file ceiling — `AGNOSAI_CX_MAX_BYTES`.
- **Float literals still compile silently.** `var x = 1.5;` gives exit 0, 96 bytes of
  output, no diagnostic, so arc B has not reached this toolchain. `agnosai_cx_compile`
  rejects the literal syntactically *before* invoking the compiler, which is the only place
  the rejection can happen.
- cx is a *bare* target: no stdlib is auto-injected, and an un-included function links to an
  undefined-fn trap (SIGILL, exit 136) rather than a link error.

**Revisit at cx arc B** (float) **and arc C** (cross-OS, caps). Until arc C, cx tool
execution is a Linux-x86-only feature and the other platforms must degrade explicitly rather
than silently.
