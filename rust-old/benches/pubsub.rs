//! Benchmarks for topic-based pub/sub with wildcard matching.

use criterion::{Criterion, criterion_group, criterion_main};
use serde_json::json;

use agnosai::orchestrator::pubsub::{PubSub, matches_pattern};

// ── matches_pattern ─────────────────────────────────────────────────────

fn bench_matches_pattern_literal(c: &mut Criterion) {
    c.bench_function("matches_pattern (literal)", |b| {
        b.iter(|| matches_pattern("task.completed", "task.completed"));
    });
}

fn bench_matches_pattern_star(c: &mut Criterion) {
    c.bench_function("matches_pattern (single *)", |b| {
        b.iter(|| matches_pattern("task.*", "task.completed"));
    });
}

fn bench_matches_pattern_hash(c: &mut Criterion) {
    c.bench_function("matches_pattern (multi-level #)", |b| {
        b.iter(|| matches_pattern("agent.#", "agent.status.health.cpu.load"));
    });
}

fn bench_matches_pattern_deep(c: &mut Criterion) {
    let topic = "a.b.c.d.e.f.g.h.i.j";
    let pattern = "a.*.c.#.h.*.j";

    c.bench_function("matches_pattern (10 segments, mixed wildcards)", |b| {
        b.iter(|| matches_pattern(pattern, topic));
    });
}

// ── PubSub::publish ─────────────────────────────────────────────────────

fn bench_publish_1_subscriber(c: &mut Criterion) {
    let ps = PubSub::new();
    let _rx = ps.subscribe("task.*");

    c.bench_function("PubSub::publish (1 subscriber)", |b| {
        b.iter(|| ps.publish("task.completed", json!({"id": 42})));
    });
}

fn bench_publish_10_subscribers(c: &mut Criterion) {
    let ps = PubSub::new();
    let _rxs: Vec<_> = (0..10)
        .map(|i| {
            // Mix of exact, wildcard, and hash patterns that all match.
            match i % 3 {
                0 => ps.subscribe("task.completed"),
                1 => ps.subscribe("task.*"),
                _ => ps.subscribe("task.#"),
            }
        })
        .collect();

    c.bench_function("PubSub::publish (10 subscribers)", |b| {
        b.iter(|| ps.publish("task.completed", json!({"id": 42})));
    });
}

fn bench_publish_100_subscribers(c: &mut Criterion) {
    let ps = PubSub::new();
    let _rxs: Vec<_> = (0..100)
        .map(|i| match i % 5 {
            0 => ps.subscribe("task.completed"),
            1 => ps.subscribe("task.*"),
            2 => ps.subscribe("task.#"),
            3 => ps.subscribe("agent.*"),
            _ => ps.subscribe("#"),
        })
        .collect();

    c.bench_function("PubSub::publish (100 subscribers)", |b| {
        b.iter(|| ps.publish("task.completed", json!({"id": 42})));
    });
}

// ── PubSub::subscribe ───────────────────────────────────────────────────

fn bench_subscribe(c: &mut Criterion) {
    let ps = PubSub::new();

    c.bench_function("PubSub::subscribe (new pattern)", |b| {
        let mut i = 0u64;
        b.iter(|| {
            let pattern = format!("topic.{i}.*");
            let _rx = ps.subscribe(&pattern);
            i += 1;
        });
    });
}

criterion_group!(
    benches,
    bench_matches_pattern_literal,
    bench_matches_pattern_star,
    bench_matches_pattern_hash,
    bench_matches_pattern_deep,
    bench_publish_1_subscriber,
    bench_publish_10_subscribers,
    bench_publish_100_subscribers,
    bench_subscribe,
);
criterion_main!(benches);
