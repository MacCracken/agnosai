//! Integration test: full Phase 1+2 pipeline exercise.
//!
//! 1. Create agent definitions programmatically
//! 2. Register echo tool in a ToolRegistry
//! 3. Build a CrewSpec with DAG tasks
//! 4. Run through CrewRunner
//! 5. Verify results come back in dependency order

use std::sync::Arc;

use agnosai::core::agent::AgentDefinition;
use agnosai::core::crew::CrewSpec;
use agnosai::core::task::{ProcessMode, Task, TaskStatus};
use agnosai::orchestrator::crew_runner::CrewRunner;
use agnosai::tools::builtin::echo::EchoTool;
use agnosai::tools::registry::ToolRegistry;

fn make_agent(key: &str, role: &str, goal: &str, tools: Vec<&str>) -> AgentDefinition {
    AgentDefinition::new(key, role, goal)
        .with_tools(tools.into_iter().map(String::from).collect())
        .with_domain("quality")
}

#[tokio::test]
async fn crew_with_tools_dag_pipeline() {
    // ── 1. Create agent definitions ─────────────────────────────────
    let analyst = make_agent(
        "analyst",
        "requirements analyst",
        "gather and clarify requirements",
        vec!["echo"],
    );
    let designer = make_agent(
        "designer",
        "system designer",
        "design the solution architecture",
        vec!["echo"],
    );
    let reviewer = make_agent(
        "reviewer",
        "quality reviewer",
        "review deliverables for correctness",
        vec!["echo"],
    );

    // ── 2. Register echo tool in ToolRegistry ───────────────────────
    let registry = ToolRegistry::new();
    registry.register(Arc::new(EchoTool));
    assert!(registry.has("echo"));
    assert_eq!(registry.count(), 1);

    // Verify the tool works standalone.
    let tool = registry.get("echo").unwrap();
    assert_eq!(tool.name(), "echo");

    // ── 3. Build CrewSpec with DAG tasks ─────────────────────────────
    let gather_reqs = Task::new("Gather and document project requirements");
    let mut design_system = Task::new("Design system architecture based on requirements");
    design_system.dependencies.push(gather_reqs.id);

    let mut review_output = Task::new("Review all deliverables for quality");
    review_output.dependencies.push(design_system.id);

    let gather_id = gather_reqs.id;
    let design_id = design_system.id;
    let review_id = review_output.id;

    let spec = CrewSpec::new("integration-test-crew")
        .with_agents(vec![analyst, designer, reviewer])
        .with_tasks(vec![gather_reqs, design_system, review_output])
        .with_process(ProcessMode::Dag);

    // ── 4. Run through CrewRunner ───────────────────────────────────
    let mut runner = CrewRunner::new(spec);
    let state = runner.run().await.expect("crew run should succeed");

    // ── 5. Verify results ───────────────────────────────────────────
    assert_eq!(
        state.status,
        agnosai::core::crew::CrewStatus::Completed,
        "crew should complete successfully"
    );
    assert_eq!(state.results.len(), 3, "should have 3 task results");

    for result in &state.results {
        assert_eq!(result.status, TaskStatus::Completed);
    }

    let pos_of = |id: uuid::Uuid| {
        state
            .results
            .iter()
            .position(|r| r.task_id == id)
            .expect("task result should exist")
    };
    let gather_pos = pos_of(gather_id);
    let design_pos = pos_of(design_id);
    let review_pos = pos_of(review_id);

    assert!(gather_pos < design_pos);
    assert!(design_pos < review_pos);

    let gather_result = state
        .results
        .iter()
        .find(|r| r.task_id == gather_id)
        .unwrap();
    assert!(
        gather_result
            .output
            .contains("Gather and document project requirements")
    );

    let design_result = state
        .results
        .iter()
        .find(|r| r.task_id == design_id)
        .unwrap();
    assert!(design_result.output.contains("Design system architecture"));

    let review_result = state
        .results
        .iter()
        .find(|r| r.task_id == review_id)
        .unwrap();
    assert!(review_result.output.contains("Review all deliverables"));
}

#[tokio::test]
async fn crew_with_tools_parallel_pipeline() {
    let agents = vec![
        make_agent("agent-a", "worker", "do work", vec!["echo"]),
        make_agent("agent-b", "worker", "do work", vec!["echo"]),
    ];

    let registry = ToolRegistry::new();
    registry.register(Arc::new(EchoTool));

    let tasks = vec![
        Task::new("parallel task 1"),
        Task::new("parallel task 2"),
        Task::new("parallel task 3"),
    ];

    let spec = CrewSpec::new("parallel-integration-crew")
        .with_agents(agents)
        .with_tasks(tasks)
        .with_process(ProcessMode::Parallel { max_concurrency: 2 });

    let mut runner = CrewRunner::new(spec);
    let state = runner.run().await.expect("crew run should succeed");

    assert_eq!(state.status, agnosai::core::crew::CrewStatus::Completed);
    assert_eq!(state.results.len(), 3);

    let outputs: std::collections::HashSet<String> =
        state.results.iter().map(|r| r.output.clone()).collect();
    assert!(outputs.contains("parallel task 1"));
    assert!(outputs.contains("parallel task 2"));
    assert!(outputs.contains("parallel task 3"));
}
