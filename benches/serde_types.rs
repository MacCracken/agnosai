//! Benchmarks for serde round-trips on core types.

use criterion::{Criterion, criterion_group, criterion_main};

use agnosai::core::resource::*;
use agnosai::core::task::*;
use agnosai::core::agent::AgentDefinition;

fn bench_hardware_inventory_json(c: &mut Criterion) {
    let mut devices = Vec::new();
    for i in 0..8 {
        devices.push(ComputeDevice {
            index: i,
            name: format!("GPU #{i}"),
            accelerator: AcceleratorType::Cuda,
            memory_total_mb: 81920,
            memory_available_mb: 81920,
        });
    }
    let inv = HardwareInventory {
        cpu_cores: 128,
        memory_total_mb: 1048576,
        devices,
    };
    let json = serde_json::to_string(&inv).unwrap();

    let mut group = c.benchmark_group("hardware_inventory_json");
    group.bench_function("serialize (8 devices)", |b| {
        b.iter(|| serde_json::to_string(&inv).unwrap());
    });
    group.bench_function("deserialize (8 devices)", |b| {
        b.iter(|| serde_json::from_str::<HardwareInventory>(&json).unwrap());
    });
    group.finish();
}

fn bench_task_json(c: &mut Criterion) {
    let mut task = Task::new("Analyze quarterly revenue data and produce a summary report");
    task.context.insert(
        "required_tools".into(),
        serde_json::json!(["sql_query", "chart_gen", "report_writer"]),
    );
    task.context
        .insert("complexity".into(), serde_json::json!("high"));
    task.context
        .insert("domain".into(), serde_json::json!("finance"));
    let json = serde_json::to_string(&task).unwrap();

    let mut group = c.benchmark_group("task_json");
    group.bench_function("serialize", |b| {
        b.iter(|| serde_json::to_string(&task).unwrap());
    });
    group.bench_function("deserialize", |b| {
        b.iter(|| serde_json::from_str::<Task>(&json).unwrap());
    });
    group.finish();
}

fn bench_agent_from_json(c: &mut Criterion) {
    let json = r#"{
        "agent_key": "data-analyst",
        "name": "Data Analyst",
        "role": "Senior data analyst",
        "goal": "Produce accurate financial reports",
        "backstory": "10 years of experience in financial data analysis",
        "tools": ["sql_query", "chart_gen", "report_writer", "data_clean"],
        "complexity": "high",
        "llm_model": "claude-sonnet-4-6",
        "gpu_required": false,
        "gpu_preferred": true,
        "hardware": {
            "accelerators": ["cuda"],
            "min_memory_mb": 16384,
            "min_device_count": 1,
            "min_cpu_cores": 4
        }
    }"#;

    c.bench_function("agent_definition_from_json", |b| {
        b.iter(|| AgentDefinition::from_json(json).unwrap());
    });
}

fn bench_resource_budget_json(c: &mut Criterion) {
    let budget = ResourceBudget {
        max_tokens: Some(100000),
        max_cost_usd: Some(5.0),
        max_duration_secs: Some(600),
        max_concurrent_tasks: Some(8),
    };
    let json = serde_json::to_string(&budget).unwrap();

    let mut group = c.benchmark_group("resource_budget_json");
    group.bench_function("serialize", |b| {
        b.iter(|| serde_json::to_string(&budget).unwrap());
    });
    group.bench_function("deserialize", |b| {
        b.iter(|| serde_json::from_str::<ResourceBudget>(&json).unwrap());
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_hardware_inventory_json,
    bench_task_json,
    bench_agent_from_json,
    bench_resource_budget_json,
);
criterion_main!(benches);
