//! Benchmarks for the learning module: capability scoring, UCB1 strategy,
//! replay buffer, Q-learning optimizer, and performance profiling.

use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};

use agnosai::learning::capability::CapabilityScorer;
use agnosai::learning::optimizer::QLearner;
use agnosai::learning::profile::PerformanceProfile;
use agnosai::learning::replay::{Experience, ReplayBuffer};
use agnosai::learning::strategy::Ucb1;

// ── CapabilityScorer ────────────────────────────────────────────────────

fn make_scorer_with_capabilities(n: usize) -> CapabilityScorer {
    let mut scorer = CapabilityScorer::new();
    for i in 0..n {
        let cap = format!("capability_{i}");
        // Mix of successes and failures so confidence varies.
        for _ in 0..(i % 7 + 1) {
            scorer.record_success(&cap);
        }
        for _ in 0..(i % 5) {
            scorer.record_failure(&cap);
        }
    }
    scorer
}

fn bench_capability_record(c: &mut Criterion) {
    let mut scorer = make_scorer_with_capabilities(50);
    let mut toggle = false;

    c.bench_function("CapabilityScorer::record (50 caps)", |b| {
        b.iter(|| {
            if toggle {
                scorer.record_success("capability_25");
            } else {
                scorer.record_failure("capability_25");
            }
            toggle = !toggle;
        });
    });
}

fn bench_capability_confidence(c: &mut Criterion) {
    let scorer = make_scorer_with_capabilities(50);
    let caps: Vec<String> = (0..50).map(|i| format!("capability_{i}")).collect();

    c.bench_function("CapabilityScorer::confidence (50 caps)", |b| {
        let mut idx = 0usize;
        b.iter(|| {
            let _ = scorer.confidence(&caps[idx % 50]);
            idx += 1;
        });
    });
}

// ── Ucb1 ────────────────────────────────────────────────────────────────

fn make_explored_bandit(n: usize) -> (Ucb1, u32) {
    let names: Vec<String> = (0..n).map(|i| format!("arm_{i}")).collect();
    let mut bandit = Ucb1::new(names);
    let rounds = (n * 10) as u32;
    for round in 0..rounds {
        let arm = round as usize % n;
        #[allow(clippy::manual_is_multiple_of)]
        let reward = if arm % 3 == 0 { 0.8 } else { 0.3 };
        bandit.update(arm, reward);
    }
    (bandit, rounds)
}

fn bench_ucb1_select_10(c: &mut Criterion) {
    let (bandit, rounds) = make_explored_bandit(10);

    c.bench_function("Ucb1::select (10 arms)", |b| {
        b.iter(|| bandit.select(rounds));
    });
}

fn bench_ucb1_select_50(c: &mut Criterion) {
    let (bandit, rounds) = make_explored_bandit(50);

    c.bench_function("Ucb1::select (50 arms)", |b| {
        b.iter(|| bandit.select(rounds));
    });
}

// ── ReplayBuffer ────────────────────────────────────────────────────────

fn make_full_buffer(size: usize) -> ReplayBuffer {
    let mut buf = ReplayBuffer::new(size);
    for i in 0..size {
        buf.push(Experience::new(
            format!("state_{}", i % 100),
            format!("action_{}", i % 10),
            (i as f64 * 0.01).sin(),
            format!("state_{}", (i + 1) % 100),
            (i as f64 * 0.1).cos().abs() + 0.01,
        ));
    }
    buf
}

fn bench_replay_push(c: &mut Criterion) {
    let mut buf = make_full_buffer(1000);
    let mut i = 1000u64;

    c.bench_function("ReplayBuffer::push (1000 cap, full)", |b| {
        b.iter(|| {
            buf.push(Experience::new(
                format!("s_{i}"),
                "act",
                1.0,
                "s_next",
                (i as f64 * 0.07).cos().abs() + 0.5,
            ));
            i += 1;
        });
    });
}

fn bench_replay_sample(c: &mut Criterion) {
    let buf = make_full_buffer(1000);

    c.bench_function("ReplayBuffer::sample(32) from 1000", |b| {
        b.iter(|| buf.sample(32));
    });
}

// ── QLearner ────────────────────────────────────────────────────────────

fn make_trained_qlearner() -> QLearner {
    let mut q = QLearner::new(0.1, 0.95);
    let actions: Vec<&str> = vec!["up", "down", "left", "right", "wait"];
    for i in 0..1000 {
        let state = format!("s_{}", i % 50);
        let next_state = format!("s_{}", (i + 1) % 50);
        let action = actions[i % actions.len()];
        let reward = if i % 7 == 0 { 1.0 } else { -0.1 };
        q.update(&state, action, reward, &next_state, &actions);
    }
    q
}

fn bench_qlearner_update(c: &mut Criterion) {
    let mut q = make_trained_qlearner();
    let actions: Vec<&str> = vec!["up", "down", "left", "right", "wait"];
    let mut i = 0usize;

    c.bench_function("QLearner::update (1000 state-actions)", |b| {
        b.iter(|| {
            let state = format!("s_{}", i % 50);
            let next_state = format!("s_{}", (i + 1) % 50);
            let action = actions[i % actions.len()];
            q.update(&state, action, 0.5, &next_state, &actions);
            i += 1;
        });
    });
}

fn bench_qlearner_best_action(c: &mut Criterion) {
    let q = make_trained_qlearner();
    let actions: Vec<&str> = vec!["up", "down", "left", "right", "wait"];
    let mut i = 0usize;

    c.bench_function("QLearner::best_action (1000 state-actions)", |b| {
        b.iter(|| {
            let state = format!("s_{}", i % 50);
            let _ = q.best_action(&state, &actions);
            i += 1;
        });
    });
}

// ── PerformanceProfile ──────────────────────────────────────────────────

fn make_profile(num_agents: usize) -> PerformanceProfile {
    let mut profile = PerformanceProfile::new();
    let action_types = ["build", "test", "deploy", "analyze", "review"];
    for a in 0..num_agents {
        let agent = format!("agent-{a}");
        for i in 0..50 {
            let action = action_types[i % action_types.len()];
            let duration = Duration::from_millis(50 + (i as u64 * 7) % 200);
            let success = i % 3 != 0;
            profile.record(&agent, action, duration, success);
        }
    }
    profile
}

fn bench_profile_record(c: &mut Criterion) {
    let mut profile = make_profile(20);

    c.bench_function("PerformanceProfile::record (20 agents)", |b| {
        let mut i = 0u64;
        b.iter(|| {
            let agent = format!("agent-{}", i % 20);
            let duration = Duration::from_millis(100 + i % 150);
            #[allow(clippy::manual_is_multiple_of)]
            profile.record(&agent, "build", duration, i % 2 == 0);
            i += 1;
        });
    });
}

fn bench_profile_success_rate(c: &mut Criterion) {
    let profile = make_profile(20);
    let agents: Vec<String> = (0..20).map(|i| format!("agent-{i}")).collect();

    c.bench_function("PerformanceProfile::success_rate (20 agents)", |b| {
        let mut idx = 0usize;
        b.iter(|| {
            let _ = profile.success_rate(&agents[idx % 20]);
            idx += 1;
        });
    });
}

criterion_group!(
    benches,
    bench_capability_record,
    bench_capability_confidence,
    bench_ucb1_select_10,
    bench_ucb1_select_50,
    bench_replay_push,
    bench_replay_sample,
    bench_qlearner_update,
    bench_qlearner_best_action,
    bench_profile_record,
    bench_profile_success_rate,
);
criterion_main!(benches);
