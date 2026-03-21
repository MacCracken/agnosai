//! Benchmarks for inter-node relay messaging.

use criterion::{Criterion, criterion_group, criterion_main};
use serde_json::json;

use agnosai::fleet::relay::{Relay, RelayMessage};
use chrono::Utc;

// ── Relay::send ─────────────────────────────────────────────────────────

fn bench_send(c: &mut Criterion) {
    let relay = Relay::new("node-a", 4096);
    let _rx = relay.subscribe();

    c.bench_function("Relay::send throughput", |b| {
        b.iter(|| {
            relay.send(
                "node-b",
                "task.assigned",
                json!({"agent": "worker-1", "payload_kb": 4}),
            );
        });
    });
}

// ── Relay::receive ──────────────────────────────────────────────────────

fn bench_receive_no_dupes(c: &mut Criterion) {
    let relay = Relay::new("node-b", 4096);
    let mut seq = 1u64;

    c.bench_function("Relay::receive (no dupes)", |b| {
        b.iter(|| {
            let msg = RelayMessage {
                seq,
                from: "node-a".into(),
                to: "node-b".into(),
                topic: "task.result".into(),
                payload: json!({"status": "ok"}),
                timestamp: Utc::now(),
            };
            relay.receive(msg);
            seq += 1;
        });
    });
}

fn bench_receive_50pct_dupes(c: &mut Criterion) {
    let relay = Relay::new("node-b", 4096);

    // Pre-seed: receive messages with seq 1..500 so they are "seen".
    for s in 1..=500u64 {
        let msg = RelayMessage {
            seq: s,
            from: "node-a".into(),
            to: "node-b".into(),
            topic: "warmup".into(),
            payload: json!(null),
            timestamp: Utc::now(),
        };
        let _ = relay.receive(msg);
    }

    let mut next_new = 501u64;
    let mut toggle = false;

    c.bench_function("Relay::receive (50% dupes)", |b| {
        b.iter(|| {
            let seq = if toggle {
                // Duplicate: re-send an already-seen sequence.
                rand::random::<u64>() % 500 + 1
            } else {
                let s = next_new;
                next_new += 1;
                s
            };
            toggle = !toggle;

            let msg = RelayMessage {
                seq,
                from: "node-a".into(),
                to: "node-b".into(),
                topic: "task.result".into(),
                payload: json!({"data": 123}),
                timestamp: Utc::now(),
            };
            relay.receive(msg);
        });
    });
}

// ── Relay::broadcast ────────────────────────────────────────────────────

fn bench_broadcast(c: &mut Criterion) {
    let relay = Relay::new("node-a", 4096);
    let _rx = relay.subscribe();

    c.bench_function("Relay::broadcast throughput", |b| {
        b.iter(|| {
            relay.broadcast("heartbeat", json!({"ts": 1234567890, "load": 0.42}));
        });
    });
}

criterion_group!(
    benches,
    bench_send,
    bench_receive_no_dupes,
    bench_receive_50pct_dupes,
    bench_broadcast,
);
criterion_main!(benches);
