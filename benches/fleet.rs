//! Benchmarks for fleet state management, coordinator fan-out, and compute scheduling.

use std::collections::HashSet;

use criterion::{Criterion, criterion_group, criterion_main};
use uuid::Uuid;

use agnosai::core::resource::AcceleratorType;
use agnosai::fleet::coordinator::FleetCoordinator;
use agnosai::fleet::gpu::ComputeScheduler;
use agnosai::fleet::state::CrewStateManager;

// ── Helpers ─────────────────────────────────────────────────────────────

fn nodes(ids: &[&str]) -> HashSet<String> {
    ids.iter().map(|s| (*s).to_string()).collect()
}

fn scheduler_8_devices() -> ComputeScheduler {
    let mut s = ComputeScheduler::new();
    for i in 0..8 {
        s.add_device(format!("GPU-{i}"), AcceleratorType::Cuda, 24000);
    }
    s
}

// ── CrewStateManager::create_run ────────────────────────────────────────

fn bench_create_run(c: &mut Criterion) {
    c.bench_function("CrewStateManager::create_run (3 nodes)", |b| {
        let mut mgr = CrewStateManager::new();
        let n = nodes(&["a", "b", "c"]);
        b.iter(|| {
            let _ = mgr.create_run(n.clone(), 10);
        });
    });
}

// ── CrewStateManager::reach_barrier ─────────────────────────────────────

fn bench_reach_barrier(c: &mut Criterion) {
    c.bench_function("CrewStateManager::reach_barrier (3 nodes)", |b| {
        let mut mgr = CrewStateManager::new();
        let n = nodes(&["a", "b", "c"]);
        let mut run_id = mgr.create_run(n.clone(), 5);
        let mut cycle = 0u64;

        b.iter(|| {
            // Each full cycle: 3 reach_barrier calls completing one barrier.
            let barrier_name = format!("sync-{cycle}");
            mgr.reach_barrier(run_id, "a".into(), &barrier_name);
            mgr.reach_barrier(run_id, "b".into(), &barrier_name);
            mgr.reach_barrier(run_id, "c".into(), &barrier_name);
            cycle += 1;

            // Recreate run periodically to avoid unbounded barrier map growth.
            if cycle.is_multiple_of(100) {
                run_id = mgr.create_run(n.clone(), 5);
            }
        });
    });
}

// ── FleetCoordinator::fan_out ───────────────────────────────────────────

fn bench_fan_out(c: &mut Criterion) {
    c.bench_function("FleetCoordinator::fan_out (10 tasks, 3 nodes)", |b| {
        let mut coord = FleetCoordinator::new();

        b.iter(|| {
            let tasks: Vec<(Uuid, String)> = (0..10)
                .map(|i| (Uuid::new_v4(), format!("task-{i}")))
                .collect();
            let assignments: Vec<(Uuid, String)> = tasks
                .iter()
                .enumerate()
                .map(|(i, (id, _))| {
                    let node = match i % 3 {
                        0 => "node-a",
                        1 => "node-b",
                        _ => "node-c",
                    };
                    (*id, node.to_string())
                })
                .collect();
            let _ = coord.fan_out(tasks, assignments);
        });
    });
}

// ── FleetCoordinator::complete_task ─────────────────────────────────────

fn bench_complete_task(c: &mut Criterion) {
    c.bench_function("FleetCoordinator::task_completed", |b| {
        let mut coord = FleetCoordinator::new();

        // Pre-create tasks to complete during the benchmark.
        let mut task_ids = Vec::new();
        for batch in 0..200 {
            let tasks: Vec<(Uuid, String)> = (0..10)
                .map(|i| (Uuid::new_v4(), format!("task-{batch}-{i}")))
                .collect();
            let assignments: Vec<(Uuid, String)> = tasks
                .iter()
                .map(|(id, _)| (*id, "node-a".to_string()))
                .collect();
            for (id, _) in &tasks {
                task_ids.push(*id);
            }
            coord.fan_out(tasks, assignments);
        }

        let mut idx = 0;
        b.iter(|| {
            if idx < task_ids.len() {
                let _ = coord.task_completed(task_ids[idx]);
                idx += 1;
            }
        });
    });
}

// ── ComputeScheduler::allocate ──────────────────────────────────────────

fn bench_compute_allocate(c: &mut Criterion) {
    c.bench_function("ComputeScheduler::allocate (8 devices)", |b| {
        let mut sched = scheduler_8_devices();
        let mut allocated = Vec::new();

        b.iter(|| {
            let id = Uuid::new_v4();
            if let Some(_alloc) = sched.allocate(id, 1000, None) {
                allocated.push(id);
            } else {
                // All full — release everything and retry.
                for old_id in allocated.drain(..) {
                    sched.release(old_id);
                }
                let _ = sched.allocate(id, 1000, None);
                allocated.push(id);
            }
        });
    });
}

// ── ComputeScheduler::release ───────────────────────────────────────────

fn bench_compute_release(c: &mut Criterion) {
    c.bench_function("ComputeScheduler::release (8 devices)", |b| {
        let mut sched = scheduler_8_devices();

        // Pre-allocate a pool of IDs to release and re-allocate.
        let mut ids: Vec<Uuid> = Vec::new();
        for _ in 0..8 {
            let id = Uuid::new_v4();
            sched.allocate(id, 20000, None);
            ids.push(id);
        }

        let mut idx = 0;
        b.iter(|| {
            let id = ids[idx % ids.len()];
            sched.release(id);
            sched.allocate(id, 20000, None);
            idx += 1;
        });
    });
}

// ── ComputeScheduler::best_device ───────────────────────────────────────

fn bench_compute_best_device(c: &mut Criterion) {
    c.bench_function("ComputeScheduler::best_device (8 devices)", |b| {
        let sched = scheduler_8_devices();
        b.iter(|| {
            let _ = sched.best_device();
        });
    });
}

criterion_group!(
    benches,
    bench_create_run,
    bench_reach_barrier,
    bench_fan_out,
    bench_complete_task,
    bench_compute_allocate,
    bench_compute_release,
    bench_compute_best_device,
);
criterion_main!(benches);
