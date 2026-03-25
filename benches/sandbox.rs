//! Benchmarks for process and WASM sandboxes.

use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};

use agnosai::sandbox::{ProcessSandbox, WasmSandbox};

// ── ProcessSandbox::execute_argv — echo latency ────────────────────────

fn bench_process_execute_argv_echo(c: &mut Criterion) {
    let sandbox = ProcessSandbox::shell(Duration::from_secs(5));

    c.bench_function("ProcessSandbox::execute_argv (echo)", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                let sb = &sandbox;
                async move {
                    let _ = sb.execute_argv(&["echo", "hello"], "").await.unwrap();
                }
            });
    });
}

// ── ProcessSandbox::execute_argv — cat stdin passthrough ───────────────

fn bench_process_execute_argv_cat_stdin(c: &mut Criterion) {
    let sandbox = ProcessSandbox::shell(Duration::from_secs(5));

    c.bench_function("ProcessSandbox::execute_argv (cat stdin)", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                let sb = &sandbox;
                async move {
                    let _ = sb
                        .execute_argv(&["cat"], "small input payload")
                        .await
                        .unwrap();
                }
            });
    });
}

// ── ProcessSandbox::execute — shell echo latency ───────────────────────

fn bench_process_execute_shell_echo(c: &mut Criterion) {
    let sandbox = ProcessSandbox::shell(Duration::from_secs(5));

    c.bench_function("ProcessSandbox::execute (shell echo)", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                let sb = &sandbox;
                async move {
                    let _ = sb.execute("echo hello", "").await.unwrap();
                }
            });
    });
}

// ── WasmSandbox::execute — minimal WASI module ────────────────────────

fn bench_wasm_execute_hello(c: &mut Criterion) {
    // Minimal WASI module that writes "hello" to stdout via fd_write.
    let wat_src = r#"(module
        (import "wasi_snapshot_preview1" "fd_write"
            (func $fd_write (param i32 i32 i32 i32) (result i32)))
        (memory (export "memory") 1)
        (data (i32.const 0) "hello")
        (data (i32.const 8) "\00\00\00\00")
        (data (i32.const 12) "\05\00\00\00")
        (func (export "_start")
            (drop (call $fd_write
                (i32.const 1)
                (i32.const 8)
                (i32.const 1)
                (i32.const 20)
            ))
        )
    )"#;

    let wasm_bytes = wat::parse_str(wat_src).expect("WAT should parse");
    let sandbox = WasmSandbox::new().expect("should create WASM sandbox");
    let module = sandbox
        .load_module(&wasm_bytes)
        .expect("should load module");

    c.bench_function("WasmSandbox::execute (hello stdout)", |b| {
        b.iter(|| {
            let _ = sandbox.execute(&module, "").unwrap();
        });
    });
}

criterion_group!(
    benches,
    bench_process_execute_argv_echo,
    bench_process_execute_argv_cat_stdin,
    bench_process_execute_shell_echo,
    bench_wasm_execute_hello,
);
criterion_main!(benches);
