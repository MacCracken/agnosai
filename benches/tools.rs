//! Benchmarks for the tool registry: registration, lookup, and listing.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};

use agnosai::tools::builtin::echo::EchoTool;
use agnosai::tools::native::{NativeTool, ToolInput, ToolOutput, ToolSchema};
use agnosai::tools::registry::ToolRegistry;

/// A thin wrapper around `EchoTool` that overrides the name, allowing us to
/// register many distinct tools without hitting `#[non_exhaustive]` struct
/// construction restrictions from outside the crate.
struct NamedEcho {
    tool_name: String,
}

impl NamedEcho {
    fn new(name: impl Into<String>) -> Self {
        Self {
            tool_name: name.into(),
        }
    }
}

impl NativeTool for NamedEcho {
    fn name(&self) -> &str {
        &self.tool_name
    }

    fn description(&self) -> &str {
        "Named echo stub for benchmarking"
    }

    fn schema(&self) -> ToolSchema {
        let mut s = EchoTool.schema();
        s.name = self.tool_name.clone();
        s
    }

    fn execute(&self, input: ToolInput) -> Pin<Box<dyn Future<Output = ToolOutput> + Send + '_>> {
        EchoTool.execute(input)
    }
}

fn make_registry(n: usize) -> ToolRegistry {
    let reg = ToolRegistry::new();
    for i in 0..n {
        reg.register(Arc::new(NamedEcho::new(format!("tool_{i}"))));
    }
    reg
}

// ── ToolRegistry::get ───────────────────────────────────────────────────

fn bench_get_5(c: &mut Criterion) {
    let reg = make_registry(5);

    c.bench_function("ToolRegistry::get (5 tools)", |b| {
        let mut i = 0usize;
        b.iter(|| {
            let name = format!("tool_{}", i % 5);
            let _ = reg.get(&name);
            i += 1;
        });
    });
}

fn bench_get_50(c: &mut Criterion) {
    let reg = make_registry(50);

    c.bench_function("ToolRegistry::get (50 tools)", |b| {
        let mut i = 0usize;
        b.iter(|| {
            let name = format!("tool_{}", i % 50);
            let _ = reg.get(&name);
            i += 1;
        });
    });
}

// ── ToolRegistry::list ──────────────────────────────────────────────────

fn bench_list_50(c: &mut Criterion) {
    let reg = make_registry(50);

    c.bench_function("ToolRegistry::list (50 tools)", |b| {
        b.iter(|| reg.list());
    });
}

// ── ToolRegistry::register ──────────────────────────────────────────────

fn bench_register(c: &mut Criterion) {
    let reg = ToolRegistry::new();
    let mut i = 0u64;

    c.bench_function("ToolRegistry::register throughput", |b| {
        b.iter(|| {
            let tool = Arc::new(NamedEcho::new(format!("bench_tool_{i}")));
            reg.register(tool);
            i += 1;
        });
    });
}

criterion_group!(
    benches,
    bench_get_5,
    bench_get_50,
    bench_list_50,
    bench_register,
);
criterion_main!(benches);
