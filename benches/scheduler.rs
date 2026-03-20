//! Benchmarks for the priority-queue scheduler and DAG operations.

use criterion::{Criterion, criterion_group, criterion_main};
use std::collections::{HashMap, HashSet};

use agnosai::core::task::{ProcessMode, Task, TaskDAG, TaskId, TaskPriority};
use agnosai::orchestrator::scheduler::Scheduler;

fn bench_enqueue_dequeue(c: &mut Criterion) {
    let mut group = c.benchmark_group("scheduler_priority");

    group.bench_function("enqueue 100 tasks", |b| {
        b.iter(|| {
            let mut sched = Scheduler::new();
            for i in 0..100 {
                let mut task = Task::new(format!("task-{i}"));
                task.priority = match i % 5 {
                    0 => TaskPriority::Background,
                    1 => TaskPriority::Low,
                    2 => TaskPriority::Normal,
                    3 => TaskPriority::High,
                    _ => TaskPriority::Critical,
                };
                sched.enqueue(task);
            }
        });
    });

    group.bench_function("dequeue 100 tasks (priority order)", |b| {
        b.iter_batched(
            || {
                let mut sched = Scheduler::new();
                for i in 0..100 {
                    let mut task = Task::new(format!("task-{i}"));
                    task.priority = match i % 5 {
                        0 => TaskPriority::Background,
                        1 => TaskPriority::Low,
                        2 => TaskPriority::Normal,
                        3 => TaskPriority::High,
                        _ => TaskPriority::Critical,
                    };
                    sched.enqueue(task);
                }
                sched
            },
            |mut sched| {
                while sched.dequeue().is_some() {}
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn build_linear_dag(n: usize) -> TaskDAG {
    let mut tasks = HashMap::new();
    let mut edges = Vec::new();
    for i in 0..n {
        let key = format!("t{i}");
        tasks.insert(key.clone(), Task::new(format!("task {i}")));
        if i > 0 {
            edges.push((format!("t{}", i - 1), key));
        }
    }
    TaskDAG {
        tasks,
        edges,
        process: ProcessMode::Dag,
    }
}

fn build_wide_dag(n: usize) -> TaskDAG {
    let mut tasks = HashMap::new();
    let mut edges = Vec::new();
    tasks.insert("root".into(), Task::new("root"));
    tasks.insert("sink".into(), Task::new("sink"));
    for i in 0..n {
        let key = format!("w{i}");
        tasks.insert(key.clone(), Task::new(format!("worker {i}")));
        edges.push(("root".into(), key.clone()));
        edges.push((key, "sink".into()));
    }
    TaskDAG {
        tasks,
        edges,
        process: ProcessMode::Dag,
    }
}

fn bench_load_dag(c: &mut Criterion) {
    let mut group = c.benchmark_group("scheduler_dag");

    let linear_50 = build_linear_dag(50);
    group.bench_function("load_dag linear (50 tasks)", |b| {
        b.iter(|| {
            let mut sched = Scheduler::new();
            sched.load_dag(&linear_50).unwrap();
        });
    });

    let linear_500 = build_linear_dag(500);
    group.bench_function("load_dag linear (500 tasks)", |b| {
        b.iter(|| {
            let mut sched = Scheduler::new();
            sched.load_dag(&linear_500).unwrap();
        });
    });

    let wide_100 = build_wide_dag(100);
    group.bench_function("load_dag wide (100 workers)", |b| {
        b.iter(|| {
            let mut sched = Scheduler::new();
            sched.load_dag(&wide_100).unwrap();
        });
    });

    group.finish();
}

fn bench_ready_tasks(c: &mut Criterion) {
    let wide = build_wide_dag(100);
    let mut sched = Scheduler::new();
    sched.load_dag(&wide).unwrap();
    let completed: HashSet<TaskId> = HashSet::new();

    c.bench_function("ready_tasks (wide DAG, 100 workers)", |b| {
        b.iter(|| sched.ready_tasks(&completed));
    });
}

criterion_group!(
    benches,
    bench_enqueue_dequeue,
    bench_load_dag,
    bench_ready_tasks,
);
criterion_main!(benches);
