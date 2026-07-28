//! Benchmarks for definition loading: builtin presets, team assembly, and JSON
//! preset parsing.

use criterion::{Criterion, criterion_group, criterion_main};

use agnosai::core::agent::AgentDefinition;
use agnosai::definitions::assembler::{TeamMember, assemble_team};
use agnosai::definitions::loader::{builtin_presets, load_preset_from_json};

// ── builtin_presets() loading time ────────────────────────────────────

fn bench_builtin_presets(c: &mut Criterion) {
    c.bench_function("builtin_presets (18 preset parse)", |b| {
        b.iter(|| {
            let presets = builtin_presets();
            assert!(!presets.is_empty());
            presets
        });
    });
}

// ── assemble_team with 5 members and 20 available agents ──────────────

fn make_agent(key: &str, role: &str, tools: Vec<&str>, _complexity: &str) -> AgentDefinition {
    AgentDefinition::new(key, role, format!("Handle {role} tasks"))
        .with_name(format!("{key} Agent"))
        .with_tools(tools.into_iter().map(String::from).collect())
}

fn bench_assemble_team(c: &mut Criterion) {
    let available: Vec<AgentDefinition> = (0..20)
        .map(|i| {
            let role = match i % 5 {
                0 => "tester",
                1 => "developer",
                2 => "reviewer",
                3 => "deployer",
                _ => "analyst",
            };
            let tools: Vec<&str> = match i % 4 {
                0 => vec!["selenium", "pytest"],
                1 => vec!["git", "cargo"],
                2 => vec!["docker", "kubectl"],
                _ => vec!["sql_query", "chart_gen"],
            };
            let complexity = match i % 3 {
                0 => "low",
                1 => "medium",
                _ => "high",
            };
            make_agent(&format!("agent-{i}"), role, tools, complexity)
        })
        .collect();

    let members: Vec<TeamMember> = vec![
        TeamMember::new("tester", vec!["selenium".into()], Some("medium".into())),
        TeamMember::new("developer", vec!["git".into(), "cargo".into()], None),
        TeamMember::new("reviewer", vec![], Some("high".into())),
        TeamMember::new("deployer", vec!["docker".into()], None),
        TeamMember::new("analyst", vec!["sql_query".into()], Some("low".into())),
    ];

    c.bench_function("assemble_team (5 members, 20 agents)", |b| {
        b.iter(|| assemble_team(&members, &available));
    });
}

// ── load_preset_from_json parsing time ────────────────────────────────

fn bench_load_preset_from_json(c: &mut Criterion) {
    let json = r#"{
        "name": "bench-preset",
        "description": "Benchmark preset with 5 agents",
        "domain": "software-engineering",
        "size": "standard",
        "version": "1.0.0",
        "agents": [
            {"agent_key": "a1", "name": "Agent 1", "role": "developer", "goal": "Write code", "tools": ["git", "cargo"], "complexity": "high"},
            {"agent_key": "a2", "name": "Agent 2", "role": "tester", "goal": "Test code", "tools": ["pytest", "selenium"], "complexity": "medium"},
            {"agent_key": "a3", "name": "Agent 3", "role": "reviewer", "goal": "Review PRs", "tools": ["git"], "complexity": "medium"},
            {"agent_key": "a4", "name": "Agent 4", "role": "deployer", "goal": "Deploy apps", "tools": ["docker", "kubectl", "helm"], "complexity": "high"},
            {"agent_key": "a5", "name": "Agent 5", "role": "analyst", "goal": "Analyze data", "tools": ["sql_query", "chart_gen"], "complexity": "low"}
        ]
    }"#;

    c.bench_function("load_preset_from_json (5 agents)", |b| {
        b.iter(|| load_preset_from_json(json).unwrap());
    });
}

criterion_group!(
    benches,
    bench_builtin_presets,
    bench_assemble_team,
    bench_load_preset_from_json,
);
criterion_main!(benches);
