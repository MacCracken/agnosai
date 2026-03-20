//! Benchmarks for agent scoring and ranking.

use criterion::{Criterion, criterion_group, criterion_main};

use agnosai::core::agent::AgentDefinition;
use agnosai::core::task::Task;
use agnosai::orchestrator::scoring::{rank_agents, score_agent};

fn make_agent(tools: Vec<&str>, complexity: &str, domain: Option<&str>) -> AgentDefinition {
    AgentDefinition {
        agent_key: "bench-agent".into(),
        name: "Bench Agent".into(),
        role: "bench".into(),
        goal: "benchmark".into(),
        backstory: None,
        domain: domain.map(|s| s.to_string()),
        tools: tools.into_iter().map(|s| s.to_string()).collect(),
        complexity: complexity.to_string(),
        llm_model: None,
        gpu_required: false,
        gpu_preferred: false,
        gpu_memory_min_mb: None,
        hardware: None,
    }
}

fn make_rich_task() -> Task {
    let mut task = Task::new("complex analytics task");
    task.context.insert(
        "required_tools".into(),
        serde_json::json!(["sql_query", "chart_gen", "report_writer", "data_clean"]),
    );
    task.context
        .insert("complexity".into(), serde_json::json!("high"));
    task.context
        .insert("domain".into(), serde_json::json!("finance"));
    task.context
        .insert("gpu_required".into(), serde_json::json!(true));
    task
}

fn bench_score_agent(c: &mut Criterion) {
    let agent = make_agent(
        vec!["sql_query", "chart_gen", "report_writer"],
        "high",
        Some("finance"),
    );
    let task = make_rich_task();

    c.bench_function("score_agent (rich context)", |b| {
        b.iter(|| score_agent(&agent, &task));
    });
}

fn bench_score_agent_minimal(c: &mut Criterion) {
    let agent = make_agent(vec![], "medium", None);
    let task = Task::new("simple task");

    c.bench_function("score_agent (no context)", |b| {
        b.iter(|| score_agent(&agent, &task));
    });
}

fn bench_rank_agents_10(c: &mut Criterion) {
    let agents: Vec<AgentDefinition> = (0..10)
        .map(|i| {
            make_agent(
                vec!["sql_query", "chart_gen"]
                    .into_iter()
                    .take(i % 3 + 1)
                    .collect(),
                match i % 3 {
                    0 => "low",
                    1 => "medium",
                    _ => "high",
                },
                if i % 2 == 0 {
                    Some("finance")
                } else {
                    None
                },
            )
        })
        .collect();
    let task = make_rich_task();

    c.bench_function("rank_agents (10 agents)", |b| {
        b.iter(|| rank_agents(&agents, &task));
    });
}

fn bench_rank_agents_100(c: &mut Criterion) {
    let agents: Vec<AgentDefinition> = (0..100)
        .map(|i| {
            make_agent(
                vec!["sql_query", "chart_gen", "data_clean"]
                    .into_iter()
                    .take(i % 4 + 1)
                    .collect(),
                match i % 3 {
                    0 => "low",
                    1 => "medium",
                    _ => "high",
                },
                match i % 4 {
                    0 => Some("finance"),
                    1 => Some("devops"),
                    _ => None,
                },
            )
        })
        .collect();
    let task = make_rich_task();

    c.bench_function("rank_agents (100 agents)", |b| {
        b.iter(|| rank_agents(&agents, &task));
    });
}

criterion_group!(
    benches,
    bench_score_agent,
    bench_score_agent_minimal,
    bench_rank_agents_10,
    bench_rank_agents_100,
);
criterion_main!(benches);
