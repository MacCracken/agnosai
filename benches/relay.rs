//! Benchmarks for inter-node relay messaging.

use criterion::{Criterion, criterion_group, criterion_main};
use serde_json::json;

use agnosai::fleet::relay::{Relay, RelayMessage};

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
            let msg = RelayMessage::new(
                seq,
                "node-a",
                "node-b",
                "task.result",
                json!({"status": "ok"}),
            );
            relay.receive(msg);
            seq += 1;
        });
    });
}

fn bench_receive_50pct_dupes(c: &mut Criterion) {
    let relay = Relay::new("node-b", 4096);

    // Pre-seed: receive messages with seq 1..500 so they are "seen".
    for s in 1..=500u64 {
        let msg = RelayMessage::new(s, "node-a", "node-b", "warmup", json!(null));
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

            let msg =
                RelayMessage::new(seq, "node-a", "node-b", "task.result", json!({"data": 123}));
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

// ── Relay::send large payload ───────────────────────────────────────────

fn bench_send_large_payload(c: &mut Criterion) {
    let relay = Relay::new("node-a", 4096);
    let _rx = relay.subscribe();
    // 64 KiB JSON payload.
    let big_data: String = "x".repeat(64 * 1024);
    let payload = json!({"data": big_data});

    c.bench_function("Relay::send (64 KiB payload)", |b| {
        b.iter(|| {
            relay.send("node-b", "large", payload.clone());
        });
    });
}

// ── Relay::stats ───────────────────────────────────────────────────────

fn bench_stats(c: &mut Criterion) {
    let relay = Relay::new("node-a", 4096);
    // Send some messages to populate stats.
    for _ in 0..100 {
        relay.send("node-b", "t", json!(null));
    }

    c.bench_function("Relay::stats (after 100 sends)", |b| {
        b.iter(|| {
            let _ = relay.stats();
        });
    });
}

criterion_group!(
    benches,
    bench_send,
    bench_send_large_payload,
    bench_receive_no_dupes,
    bench_receive_50pct_dupes,
    bench_broadcast,
    bench_stats,
);
criterion_main!(benches);
