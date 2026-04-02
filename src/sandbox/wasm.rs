//! wasmtime WASM sandbox for tool execution.
//!
//! Provides isolated WASM module loading and execution using wasmtime with WASI
//! preview 1 support. Modules run with no filesystem, no network, no env vars —
//! communication happens via stdin (JSON in) and stdout (JSON out).

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use crate::core::error::AgnosaiError;
use tracing::{debug, info, warn};
use wasmtime::*;
use wasmtime_wasi::WasiCtxBuilder;
use wasmtime_wasi::p1::{self, WasiP1Ctx};
use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};

/// Default memory limit: 64 MiB.
const DEFAULT_MAX_MEMORY_BYTES: usize = 64 * 1024 * 1024;

/// Default execution timeout.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Default fuel budget (approximate instruction count).
const DEFAULT_FUEL: u64 = 1_000_000_000;

/// stdout capture buffer capacity.
const STDOUT_CAPACITY: usize = 1024 * 1024; // 1 MiB

/// A sandboxed WASM execution environment.
///
/// Each `WasmSandbox` owns a wasmtime [`Engine`] configured with:
/// - fuel-based CPU limiting
/// - epoch-based timeout interruption
/// - memory limits via [`StoreLimits`]
pub struct WasmSandbox {
    engine: Engine,
    max_memory_bytes: usize,
    timeout: Duration,
    fuel: u64,
}

/// A compiled WASM module ready for execution.
#[derive(Debug)]
pub struct WasmModule {
    module: Module,
}

/// Result of executing a WASM module.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct WasmResult {
    /// Captured stdout from the module.
    pub stdout: String,
    /// Exit code (0 = success).
    pub exit_code: i32,
}

fn sandbox_err(msg: impl Into<String>) -> AgnosaiError {
    AgnosaiError::Sandbox(msg.into())
}

fn map_wasm_err(err: impl std::fmt::Display) -> AgnosaiError {
    sandbox_err(err.to_string())
}

impl WasmSandbox {
    /// Create a sandbox with default limits (64 MiB memory, 30 s timeout).
    pub fn new() -> crate::core::Result<Self> {
        Self::with_limits(DEFAULT_MAX_MEMORY_BYTES, DEFAULT_TIMEOUT)
    }

    /// Create a sandbox with custom memory and timeout limits.
    pub fn with_limits(max_memory_bytes: usize, timeout: Duration) -> crate::core::Result<Self> {
        let mut config = Config::new();
        // Fuel-based CPU limiting — each WASM instruction consumes fuel.
        config.consume_fuel(true);
        // Epoch-based interruption for wall-clock timeout enforcement.
        config.epoch_interruption(true);

        let engine = Engine::new(&config).map_err(map_wasm_err)?;

        info!(
            max_memory_mb = max_memory_bytes / (1024 * 1024),
            timeout_secs = timeout.as_secs(),
            "WASM sandbox created"
        );

        Ok(Self {
            engine,
            max_memory_bytes,
            timeout,
            fuel: DEFAULT_FUEL,
        })
    }

    /// Load a WASM module from raw bytes.
    pub fn load_module(&self, wasm_bytes: &[u8]) -> crate::core::Result<WasmModule> {
        debug!(bytes = wasm_bytes.len(), "loading WASM module from bytes");
        let module = Module::new(&self.engine, wasm_bytes).map_err(map_wasm_err)?;
        Ok(WasmModule { module })
    }

    /// Load a WASM module from a file path.
    pub fn load_module_from_file(&self, path: &Path) -> crate::core::Result<WasmModule> {
        debug!(path = %path.display(), "loading WASM module from file");
        let module = Module::from_file(&self.engine, path).map_err(map_wasm_err)?;
        Ok(WasmModule { module })
    }

    /// Execute a loaded WASI module.
    ///
    /// The `input` string is written to the module's stdin; the module's stdout
    /// is captured and returned in [`WasmResult`]. The module must export a
    /// WASI `_start` function.
    ///
    /// The sandbox enforces:
    /// - No filesystem access
    /// - No network access
    /// - No environment variables
    /// - Memory capped at the configured limit
    /// - CPU capped via fuel
    /// - Wall-clock timeout via epoch interruption
    pub fn execute(&self, module: &WasmModule, input: &str) -> crate::core::Result<WasmResult> {
        debug!(input_len = input.len(), "executing WASM module");

        // -- Build WASI context: stdin piped, stdout captured, nothing else. --
        let stdin = MemoryInputPipe::new(input.to_owned());
        let stdout = MemoryOutputPipe::new(STDOUT_CAPACITY);
        let stdout_clone = stdout.clone();

        let wasi_ctx = WasiCtxBuilder::new().stdin(stdin).stdout(stdout).build_p1();

        // -- Build store with resource limits and fuel. --
        let limits = StoreLimitsBuilder::new()
            .memory_size(self.max_memory_bytes)
            .instances(10)
            .tables(10)
            .memories(10)
            .trap_on_grow_failure(true)
            .build();

        let mut store = Store::new(&self.engine, SandboxState { wasi_ctx, limits });
        store.limiter(|state| &mut state.limits);
        store.set_fuel(self.fuel).map_err(map_wasm_err)?;

        // Configure epoch deadline — 1 tick = timeout.
        store.epoch_deadline_trap();
        store.set_epoch_deadline(1);

        // -- Start epoch ticker in a background thread. --
        let engine = self.engine.clone();
        let timeout = self.timeout;
        let ticker = start_epoch_ticker(engine, timeout);

        // -- Link WASI and instantiate. --
        let mut linker: Linker<SandboxState> = Linker::new(&self.engine);
        p1::add_to_linker_sync(&mut linker, |state: &mut SandboxState| &mut state.wasi_ctx)
            .map_err(map_wasm_err)?;

        let instance = linker
            .instantiate(&mut store, &module.module)
            .map_err(map_wasm_err)?;

        // -- Invoke _start (WASI entry point). --
        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .map_err(|_| sandbox_err("module does not export a WASI _start function"))?;

        let exit_code = match start.call(&mut store, ()) {
            Ok(()) => 0,
            Err(err) => extract_exit_code(&err),
        };

        // Stop the ticker.
        drop(ticker);

        // -- Collect stdout. --
        let raw_stdout = stdout_clone.contents();
        let stdout_str = String::from_utf8(raw_stdout.to_vec()).unwrap_or_else(|e| {
            warn!("WASM stdout contained invalid UTF-8, lossy conversion applied");
            String::from_utf8_lossy(&e.into_bytes()).into_owned()
        });

        debug!(
            exit_code,
            stdout_len = stdout_str.len(),
            "WASM execution complete"
        );

        Ok(WasmResult {
            stdout: stdout_str,
            exit_code,
        })
    }
}

/// Store-level state carrying the WASI context and resource limits.
struct SandboxState {
    wasi_ctx: WasiP1Ctx,
    limits: StoreLimits,
}

/// Extract an exit code from a wasmtime trap/error.
///
/// WASI `proc_exit(0)` raises `I32Exit(0)` which we treat as success.
fn extract_exit_code(err: &Error) -> i32 {
    // wasmtime-wasi surfaces proc_exit as an I32Exit.
    if let Some(exit) = err.downcast_ref::<wasmtime_wasi::I32Exit>() {
        return exit.0;
    }
    // Stringify the full error chain (includes Caused-by sections).
    let full = format!("{err:?}");
    // Epoch interruption means we hit the timeout.
    // wasmtime surfaces this as "wasm trap: interrupt" in the error chain.
    if full.contains("epoch") || full.contains("interrupt") {
        warn!("WASM execution interrupted by epoch deadline (timeout)");
        return -1;
    }
    // Fuel exhaustion.
    // wasmtime surfaces this as "all fuel consumed" in the error chain.
    if full.contains("fuel") {
        warn!("WASM execution ran out of fuel (CPU limit)");
        return -2;
    }
    warn!(error = %err, "WASM execution failed");
    -3
}

/// RAII handle for the epoch-ticker thread.
struct EpochTicker {
    handle: Option<std::thread::JoinHandle<()>>,
    cancel: Arc<std::sync::atomic::AtomicBool>,
}

impl Drop for EpochTicker {
    fn drop(&mut self) {
        self.cancel
            .store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

/// Spawn a thread that increments the engine epoch after `timeout`.
fn start_epoch_ticker(engine: Engine, timeout: Duration) -> EpochTicker {
    let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let cancel_clone = cancel.clone();

    let handle = std::thread::spawn(move || {
        // Sleep in small increments so we can cancel promptly.
        let step = Duration::from_millis(50);
        let mut elapsed = Duration::ZERO;
        while elapsed < timeout {
            std::thread::sleep(step);
            if cancel_clone.load(std::sync::atomic::Ordering::Relaxed) {
                return;
            }
            elapsed += step;
        }
        engine.increment_epoch();
    });

    EpochTicker {
        handle: Some(handle),
        cancel,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_sandbox_with_defaults() {
        let sandbox = WasmSandbox::new().expect("should create sandbox");
        assert_eq!(sandbox.max_memory_bytes, DEFAULT_MAX_MEMORY_BYTES);
        assert_eq!(sandbox.timeout, DEFAULT_TIMEOUT);
    }

    #[test]
    fn create_sandbox_with_custom_limits() {
        let max_mem = 32 * 1024 * 1024;
        let timeout = Duration::from_secs(10);
        let sandbox = WasmSandbox::with_limits(max_mem, timeout).expect("should create sandbox");
        assert_eq!(sandbox.max_memory_bytes, max_mem);
        assert_eq!(sandbox.timeout, timeout);
    }

    #[test]
    fn load_invalid_bytes_returns_error() {
        let sandbox = WasmSandbox::new().expect("should create sandbox");
        let result = sandbox.load_module(b"not valid wasm");
        assert!(result.is_err());
        let err = result.unwrap_err();
        match &err {
            AgnosaiError::Sandbox(msg) => {
                assert!(
                    msg.contains("expected"),
                    "error should mention expected magic: {msg}"
                );
            }
            other => panic!("expected Sandbox error, got: {other:?}"),
        }
    }

    #[test]
    fn load_empty_bytes_returns_error() {
        let sandbox = WasmSandbox::new().expect("should create sandbox");
        let result = sandbox.load_module(b"");
        assert!(result.is_err());
    }

    #[test]
    fn load_module_from_nonexistent_file_returns_error() {
        let sandbox = WasmSandbox::new().expect("should create sandbox");
        let result = sandbox.load_module_from_file(Path::new("/nonexistent/module.wasm"));
        assert!(result.is_err());
    }

    #[test]
    fn execute_wasi_module_captures_stdout() {
        // Minimal WASI module that writes "hello" to stdout via fd_write.
        let wat = r#"(module
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

        let wasm_bytes = wat::parse_str(wat).expect("WAT should parse");
        let sandbox = WasmSandbox::new().expect("should create sandbox");
        let module = sandbox
            .load_module(&wasm_bytes)
            .expect("should load module");
        let result = sandbox.execute(&module, "").expect("should execute module");

        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout, "hello");
    }

    #[test]
    fn execute_wasi_module_with_stdin_input() {
        // Module that reads from stdin and writes what it read to stdout.
        // We keep it simple: read a fixed number of bytes and echo them.
        let wat = r#"(module
            (import "wasi_snapshot_preview1" "fd_read"
                (func $fd_read (param i32 i32 i32 i32) (result i32)))
            (import "wasi_snapshot_preview1" "fd_write"
                (func $fd_write (param i32 i32 i32 i32) (result i32)))
            (memory (export "memory") 1)
            (func (export "_start")
                ;; Set up iov for reading: buffer at offset 100, length 5
                (i32.store (i32.const 0) (i32.const 100))  ;; iov_base
                (i32.store (i32.const 4) (i32.const 5))    ;; iov_len

                ;; fd_read(stdin=0, iovs=0, iovs_count=1, nread_ptr=200)
                (drop (call $fd_read
                    (i32.const 0)
                    (i32.const 0)
                    (i32.const 1)
                    (i32.const 200)
                ))

                ;; Now write what we read: set up write iov at offset 0
                ;; pointing to the buffer at 100, with length from nread at 200
                (i32.store (i32.const 0) (i32.const 100))         ;; iov_base
                (i32.store (i32.const 4) (i32.load (i32.const 200))) ;; iov_len = nread

                ;; fd_write(stdout=1, iovs=0, iovs_count=1, nwritten_ptr=204)
                (drop (call $fd_write
                    (i32.const 1)
                    (i32.const 0)
                    (i32.const 1)
                    (i32.const 204)
                ))
            )
        )"#;

        let wasm_bytes = wat::parse_str(wat).expect("WAT should parse");
        let sandbox = WasmSandbox::new().expect("should create sandbox");
        let module = sandbox
            .load_module(&wasm_bytes)
            .expect("should load module");
        let result = sandbox
            .execute(&module, "world")
            .expect("should execute module");

        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout, "world");
    }

    #[test]
    fn fuel_exhaustion_returns_error() {
        // Infinite loop — will exhaust fuel quickly.
        let wat = r#"(module
            (func (export "_start")
                (loop br 0)
            )
            (memory (export "memory") 1)
        )"#;

        let wasm_bytes = wat::parse_str(wat).expect("WAT should parse");
        let mut sandbox =
            WasmSandbox::with_limits(DEFAULT_MAX_MEMORY_BYTES, Duration::from_secs(30))
                .expect("should create sandbox");
        // Set fuel very low so the infinite loop exhausts it before the timeout.
        sandbox.fuel = 1_000;

        let module = sandbox
            .load_module(&wasm_bytes)
            .expect("should load module");
        let result = sandbox
            .execute(&module, "")
            .expect("should return result, not panic");

        assert_eq!(
            result.exit_code, -2,
            "fuel exhaustion should produce exit_code -2, got {}",
            result.exit_code
        );
    }

    #[test]
    fn epoch_timeout_returns_error() {
        // Infinite loop — will be interrupted by epoch timeout.
        let wat = r#"(module
            (func (export "_start")
                (loop br 0)
            )
            (memory (export "memory") 1)
        )"#;

        let wasm_bytes = wat::parse_str(wat).expect("WAT should parse");
        let mut sandbox =
            WasmSandbox::with_limits(DEFAULT_MAX_MEMORY_BYTES, Duration::from_millis(100))
                .expect("should create sandbox");
        // Give plenty of fuel so it doesn't run out before the epoch fires.
        sandbox.fuel = u64::MAX / 2;

        let module = sandbox
            .load_module(&wasm_bytes)
            .expect("should load module");
        let result = sandbox
            .execute(&module, "")
            .expect("should return result, not panic");

        assert_eq!(
            result.exit_code, -1,
            "epoch timeout should produce exit_code -1, got {}",
            result.exit_code
        );
    }

    #[test]
    fn execute_valid_wasi_module_exits_cleanly() {
        // Minimal module that simply returns (exits with code 0).
        let wat = r#"(module
            (memory (export "memory") 1)
            (func (export "_start"))
        )"#;

        let wasm_bytes = wat::parse_str(wat).expect("WAT should parse");
        let sandbox = WasmSandbox::new().expect("should create sandbox");
        let module = sandbox
            .load_module(&wasm_bytes)
            .expect("should load module");
        let result = sandbox.execute(&module, "").expect("should execute module");

        assert_eq!(result.exit_code, 0, "clean exit should produce exit_code 0");
        assert!(
            result.stdout.is_empty(),
            "no-op module should produce empty stdout"
        );
    }

    #[test]
    fn zero_length_input_succeeds() {
        // Module that reads stdin — with empty input, fd_read returns 0 bytes.
        let wat = r#"(module
            (import "wasi_snapshot_preview1" "fd_read"
                (func $fd_read (param i32 i32 i32 i32) (result i32)))
            (memory (export "memory") 1)
            (func (export "_start")
                ;; Set up iov: buffer at 100, length 16
                (i32.store (i32.const 0) (i32.const 100))
                (i32.store (i32.const 4) (i32.const 16))

                ;; fd_read(stdin=0, iovs=0, iovs_count=1, nread_ptr=200)
                (drop (call $fd_read
                    (i32.const 0)
                    (i32.const 0)
                    (i32.const 1)
                    (i32.const 200)
                ))
            )
        )"#;

        let wasm_bytes = wat::parse_str(wat).expect("WAT should parse");
        let sandbox = WasmSandbox::new().expect("should create sandbox");
        let module = sandbox
            .load_module(&wasm_bytes)
            .expect("should load module");
        let result = sandbox
            .execute(&module, "")
            .expect("should execute with empty input");

        assert_eq!(result.exit_code, 0, "empty input should not cause an error");
    }
}
