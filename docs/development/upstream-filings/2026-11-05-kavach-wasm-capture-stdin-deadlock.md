# kavach: the WASM capture deadlocks when a guest writes while it reads stdin, and the deadline never fires

**Found by:** agnosai (2026-11-05, while re-checking its roadmap B16 against kavach 3.14.2).
**Version:** kavach 3.14.2, wasmtime 50.0.1, cyrius 6.7.18.

## Summary

`confine_capture_input` (the WASM backend's capture) writes the guest's whole stdin with one blocking
`sys_write`, on a plain pipe, **before** it starts draining stdout and before its deadline loop. A guest that
reads all of its input and only then writes is fine. A guest that writes as it reads fills its stdout pipe
while kavach is still blocked writing stdin. From then on, kavach waits on stdin, wasmtime waits on stdout,
and **`timeout_ms` never fires**, because the deadline is only checked in the drain loop kavach never reaches.

## Repro

The probe runs the guest through `sandbox_exec` with N bytes of stdin and a 4 s timeout:

```cyr
include "lib/syscalls.cyr"
include "lib/alloc.cyr"
include "lib/args.cyr"
include "lib/kavach.cyr"

fn _w(c) { syscall(1, 1, c, strlen(c)); return 0; }

fn main() {
    alloc_init();
    args_init();
    var path = argv(1);
    var n = atoi(argv(2));
    kavach_init();
    signal_ignore(Signal.SIGPIPE);
    var input = alloc(n + 1);
    var i = 0;
    while (i < n) { store8(input + i, 65 + (i % 26)); i = i + 1; }
    store8(input + n, 0);
    var cfg = config_new();
    config_backend(cfg, KavachBackend.WASM);
    config_timeout_ms(cfg, 4000);
    var pol = policy_basic();
    SandboxPolicy_set_memory_limit_mb(pol, 64);
    config_policy(cfg, pol);
    config_stdin(cfg, input, n);
    config_fuel(cfg, 1000000000);
    var sb = sandbox_create(cfg);
    sandbox_transition(sb, SandboxState.RUNNING);
    var t0 = mono_now_ns();
    var r = sandbox_exec(sb, path);
    var ms = (mono_now_ns() - t0) / 1000000;
    if (r == 0) { _w("exec returned 0\n"); return 0; }
    _w(path); _w(" in="); print_num(n);
    _w(" out="); print_num(strlen(ExecResult_stdout(r)));
    _w(" exit="); print_num(ExecResult_exit_code(r));
    _w(" timed_out="); print_num(ExecResult_timed_out(r));
    _w(" ms="); print_num(ms); _w("\n");
    return 0;
}
var r = main();
syscall(60, 0);
```

The guest echoes stdin to stdout in 4 KiB chunks:

```wat
(module
  (import "wasi_snapshot_preview1" "fd_read" (func $fd_read (param i32 i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (func (export "_start")
    (local $n i32)
    (block $done
      (loop $rd
        (i32.store (i32.const 0) (i32.const 1024))
        (i32.store (i32.const 4) (i32.const 4096))
        (drop (call $fd_read (i32.const 0) (i32.const 0) (i32.const 1) (i32.const 16)))
        (local.set $n (i32.load (i32.const 16)))
        (br_if $done (i32.eqz (local.get $n)))
        (i32.store (i32.const 0) (i32.const 1024))
        (i32.store (i32.const 4) (local.get $n))
        (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 16)))
        (br $rd)))))
```

`wat2wasm stream.wat`, then `./kv_probe stream.wasm <N>` under an outer `timeout 20`.

## Observed

| guest | input | result |
|---|---|---|
| reads all, then writes | 65,536 / 100,000 / 200,000 B | works |
| reads all, then writes | 1 MiB | 1,048,575 B back, cut by the 1 MiB stdout cap |
| echoes while reading | 100,000 B | works |
| echoes while reading | **200,000 B** | **hangs until the outer 20 s timeout**; kavach's 4 s `timeout_ms` never fires |

The threshold is about two pipe buffers (64 KiB each by default): the stdin pipe and the stdout pipe both
fill.

## Expected

Either the input is delivered while stdout is drained (a non-blocking stdin write interleaved with the drain
loop, or a writer thread), or at least the deadline covers the stdin write, so a stuck guest is killed at
`timeout_ms` like any other. kavach's CHANGELOG 3.11.8 and `config_stdin`'s comment say a large input should
be passed by file. Nothing refuses a large one, though, and the failure mode is an unbounded hang, not an error.

## Impact on agnosai

agnosai passes a WASM tool's parameters on stdin (`{"parameters": …}`, `src/tools/wasm_tool.cyr`). A tool that
streams its output while reading, given a large enough parameter payload (the request ceiling is 10 MiB),
holds a pool worker for good. Until this is fixed, agnosai can refuse tool input over 64 KiB, or stage it in
a file (its roadmap B9).
