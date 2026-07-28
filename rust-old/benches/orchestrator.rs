//! Benchmarks for crew orchestration: creation, sequential/parallel execution,
//! and agent scoring within the crew runner pipeline.

use criterion::{Criterion, criterion_group, criterion_main};

use agnosai::core::ResourceBudget;
use agnosai::core::agent::AgentDefinition;
use agnosai::core::crew::CrewSpec;
use agnosai::core::task::{ProcessMode, Task};
use agnosai::orchestrator::Orchestrator;
use agnosai::orchestrator::scoring::rank_agents;

fn make_agent(
    key: &str,
    tools: Vec<&str>,
    complexity: &str,
    domain: Option<&str>,
) -> AgentDefinition {
    let mut agent = AgentDefinition::new(key, "worker", "execute tasks")
        .with_name(format!("Agent {key}"))
        .with_tools(tools.into_iter().map(|s| s.to_string()).collect());
    agent.complexity = complexity.to_string();
    if let Some(d) = domain {
        agent = agent.with_domain(d);
    }
    agent
}

fn make_task(desc: &str) -> Task {
    let mut task = Task::new(desc);
    task.context
        .insert("complexity".into(), serde_json::json!("medium"));
    task.context.insert(
        "required_tools".into(),
        serde_json::json!(["sql_query", "report_writer"]),
    );
    task
}

fn make_spec(name: &str, n_tasks: usize, mode: ProcessMode) -> CrewSpec {
    let agents = vec![
        make_agent(
            "a1",
            vec!["sql_query", "report_writer"],
            "medium",
            Some("finance"),
        ),
        make_agent("a2", vec!["docker", "kubectl"], "high", Some("devops")),
        make_agent("a3", vec!["sql_query"], "low", None),
    ];
    let tasks: Vec<Task> = (0..n_tasks)
        .map(|i| make_task(&format!("task-{i}")))
        .collect();
    CrewSpec::new(name)
        .with_agents(agents)
        .with_tasks(tasks)
        .with_process(mode)
}

// ── Orchestrator::new ─────────────────────────────────────────────────

fn bench_orchestrator_new(c: &mut Criterion) {
    c.bench_function("Orchestrator::new", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let _ = Orchestrator::new(ResourceBudget::default()).await.unwrap();
            });
    });
}

// ── run_crew: 1 task, sequential (placeholder mode) ───────────────────

fn bench_run_crew_1_task(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let orch = rt
        .block_on(Orchestrator::new(ResourceBudget::default()))
        .unwrap();

    c.bench_function("run_crew (1 task, sequential, placeholder)", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                let spec = make_spec("bench-1", 1, ProcessMode::Sequential);
                let orch_ref = &orch;
                async move {
                    let _ = orch_ref.run_crew(spec).await.unwrap();
                }
            });
    });
}

// ── run_crew: 10 tasks, sequential ────────────────────────────────────

fn bench_run_crew_10_sequential(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let orch = rt
        .block_on(Orchestrator::new(ResourceBudget::default()))
        .unwrap();

    c.bench_function("run_crew (10 tasks, sequential, placeholder)", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                let spec = make_spec("bench-10-seq", 10, ProcessMode::Sequential);
                let orch_ref = &orch;
                async move {
                    let _ = orch_ref.run_crew(spec).await.unwrap();
                }
            });
    });
}

// ── run_crew: 10 tasks, parallel (max_concurrency 4) ──────────────────

fn bench_run_crew_10_parallel(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let orch = rt
        .block_on(Orchestrator::new(ResourceBudget::default()))
        .unwrap();

    c.bench_function("run_crew (10 tasks, parallel max_concurrency=4)", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                let spec = make_spec(
                    "bench-10-par",
                    10,
                    ProcessMode::Parallel { max_concurrency: 4 },
                );
                let orch_ref = &orch;
                async move {
                    let _ = orch_ref.run_crew(spec).await.unwrap();
                }
            });
    });
}

// ── pick_best_agent via scoring: 10 agents, 1 task ────────────────────

fn bench_pick_best_agent_10(c: &mut Criterion) {
    let agents: Vec<AgentDefinition> = (0..10)
        .map(|i| {
            make_agent(
                &format!("agent-{i}"),
                vec!["sql_query", "report_writer", "docker"]
                    .into_iter()
                    .take(i % 4 + 1)
                    .collect(),
                match i % 3 {
                    0 => "low",
                    1 => "medium",
                    _ => "high",
                },
                if i % 2 == 0 { Some("finance") } else { None },
            )
        })
        .collect();
    let task = make_task("complex analytics query");

    c.bench_function("pick_best_agent (10 agents, rank + select)", |b| {
        b.iter(|| {
            let ranked = rank_agents(&agents, &task);
            // Simulate pick_best_agent: take highest scorer.
            let _ = ranked.first().map(|(idx, _)| &agents[*idx]);
        });
    });
}

criterion_group!(
    benches,
    bench_orchestrator_new,
    bench_run_crew_1_task,
    bench_run_crew_10_sequential,
    bench_run_crew_10_parallel,
    bench_pick_best_agent_10,
);
criterion_main!(benches);
