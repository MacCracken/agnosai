//! Benchmarks for the human-in-the-loop approval gate: request creation
//! and decision submission throughput.

use criterion::{Criterion, criterion_group, criterion_main};

use agnosai::orchestrator::approval::{ApprovalDecision, ApprovalGate};

// ── ApprovalGate::request_approval throughput ──────────────────────────

fn bench_request_approval(c: &mut Criterion) {
    c.bench_function("ApprovalGate::request_approval", |b| {
        let gate = ApprovalGate::new();
        b.iter(|| {
            let task_id = uuid::Uuid::new_v4();
            let rx = gate.request_approval(task_id);
            assert!(rx.is_some());
            // Cancel immediately so we don't exhaust the pending limit.
            gate.cancel(task_id);
        });
    });
}

// ── ApprovalGate::submit_decision throughput ───────────────────────────

fn bench_submit_decision(c: &mut Criterion) {
    c.bench_function("ApprovalGate::submit_decision", |b| {
        let gate = ApprovalGate::new();
        b.iter(|| {
            let task_id = uuid::Uuid::new_v4();
            let _rx = gate.request_approval(task_id).unwrap();
            let delivered = gate.submit_decision(task_id, ApprovalDecision::Approved);
            assert!(delivered);
        });
    });
}

criterion_group!(benches, bench_request_approval, bench_submit_decision,);
criterion_main!(benches);
