//! Benchmarks for LLM task-complexity routing, model defaults, and complexity
//! parsing.

use criterion::{Criterion, criterion_group, criterion_main};

use agnosai::llm::router::{
    Complexity, ModelTier, TaskProfile, TaskType, default_model, parse_complexity, route,
};

// ── route() for every TaskType × Complexity combination ───────────────

fn bench_route_all_combinations(c: &mut Criterion) {
    let task_types = [
        TaskType::Summarize,
        TaskType::Classify,
        TaskType::Code,
        TaskType::Plan,
        TaskType::Reason,
        TaskType::Research,
        TaskType::MultiStep,
    ];
    let complexities = [Complexity::Simple, Complexity::Medium, Complexity::Complex];

    // Pre-build all profiles.
    let profiles: Vec<TaskProfile> = task_types
        .iter()
        .flat_map(|&tt| complexities.iter().map(move |&cx| TaskProfile::new(tt, cx)))
        .collect();

    let mut group = c.benchmark_group("llm_router_route");

    group.bench_function("route (all 21 combinations)", |b| {
        b.iter(|| {
            for p in &profiles {
                let _ = route(p);
            }
        });
    });

    // Individual hot-path samples.
    let fast_profile = TaskProfile::new(TaskType::Summarize, Complexity::Simple);
    group.bench_function("route (Summarize × Simple)", |b| {
        b.iter(|| route(&fast_profile));
    });

    let premium_profile = TaskProfile::new(TaskType::Research, Complexity::Complex);
    group.bench_function("route (Research × Complex)", |b| {
        b.iter(|| route(&premium_profile));
    });

    group.finish();
}

// ── default_model() for each tier ────────────────────────────────────

fn bench_default_model(c: &mut Criterion) {
    let mut group = c.benchmark_group("llm_router_default_model");

    group.bench_function("default_model (Fast)", |b| {
        b.iter(|| default_model(ModelTier::Fast));
    });

    group.bench_function("default_model (Capable)", |b| {
        b.iter(|| default_model(ModelTier::Capable));
    });

    group.bench_function("default_model (Premium)", |b| {
        b.iter(|| default_model(ModelTier::Premium));
    });

    group.finish();
}

// ── parse_complexity() for each variant ──────────────────────────────

fn bench_parse_complexity(c: &mut Criterion) {
    let mut group = c.benchmark_group("llm_router_parse_complexity");

    let inputs = [
        "low", "simple", "medium", "high", "complex", "HIGH", "unknown",
    ];

    for input in &inputs {
        group.bench_function(format!("parse_complexity(\"{input}\")"), |b| {
            b.iter(|| parse_complexity(input));
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_route_all_combinations,
    bench_default_model,
    bench_parse_complexity,
);
criterion_main!(benches);
