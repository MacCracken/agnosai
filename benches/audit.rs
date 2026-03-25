//! Benchmarks for the cryptographic audit chain: record throughput, verification,
//! and recent-entry retrieval.

use criterion::{Criterion, criterion_group, criterion_main};

use agnosai::llm::AuditChain;

fn make_chain(capacity: usize) -> AuditChain {
    AuditChain::new(b"bench-signing-key-32-bytes-long!", capacity)
}

// ── AuditChain::record throughput (single-threaded) ───────────────────

fn bench_record_throughput(c: &mut Criterion) {
    let chain = make_chain(100_000);

    c.bench_function("AuditChain::record throughput", |b| {
        let mut i = 0u64;
        b.iter(|| {
            chain.record(
                "task_completed",
                "info",
                &format!("task {i} completed"),
                None,
                None,
                Some(serde_json::json!({ "task_id": i, "status": "ok" })),
            );
            i += 1;
        });
    });
}

// ── AuditChain::verify on chain of 100 entries ───────────────────────

fn bench_verify_100(c: &mut Criterion) {
    let chain = make_chain(200);
    for i in 0..100 {
        chain.record("event", "info", &format!("entry {i}"), None, None, None);
    }

    c.bench_function("AuditChain::verify (100 entries)", |b| {
        b.iter(|| {
            let (valid, _err) = chain.verify();
            assert!(valid);
        });
    });
}

// ── AuditChain::verify on chain of 1000 entries ──────────────────────

fn bench_verify_1000(c: &mut Criterion) {
    let chain = make_chain(2_000);
    for i in 0..1_000 {
        chain.record("event", "info", &format!("entry {i}"), None, None, None);
    }

    c.bench_function("AuditChain::verify (1000 entries)", |b| {
        b.iter(|| {
            let (valid, _err) = chain.verify();
            assert!(valid);
        });
    });
}

// ── AuditChain::recent retrieval ─────────────────────────────────────

fn bench_recent(c: &mut Criterion) {
    let mut group = c.benchmark_group("audit_recent");

    // Chain with 1000 entries, retrieve last 10.
    let chain = make_chain(2_000);
    for i in 0..1_000 {
        chain.record("event", "info", &format!("entry {i}"), None, None, None);
    }

    group.bench_function("recent(10) from 1000 entries", |b| {
        b.iter(|| {
            let entries = chain.recent(10);
            assert_eq!(entries.len(), 10);
        });
    });

    group.bench_function("recent(100) from 1000 entries", |b| {
        b.iter(|| {
            let entries = chain.recent(100);
            assert_eq!(entries.len(), 100);
        });
    });

    group.bench_function("recent(1000) from 1000 entries", |b| {
        b.iter(|| {
            let entries = chain.recent(1000);
            assert_eq!(entries.len(), 1000);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_record_throughput,
    bench_verify_100,
    bench_verify_1000,
    bench_recent,
);
criterion_main!(benches);
