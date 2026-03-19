use agnosai_core::agent::AgentDefinition;
use agnosai_core::task::Task;

const WEIGHT_TOOL_COVERAGE: f64 = 0.40;
const WEIGHT_COMPLEXITY: f64 = 0.30;
const WEIGHT_GPU: f64 = 0.15;
const WEIGHT_DOMAIN: f64 = 0.15;

/// Map a complexity string to a numeric level.
fn complexity_level(s: &str) -> u8 {
    match s.to_lowercase().as_str() {
        "low" => 1,
        "medium" => 2,
        "high" => 3,
        _ => 2, // default to medium
    }
}

/// Score the fraction of required tools the agent provides.
fn tool_coverage_score(agent: &AgentDefinition, task: &Task) -> f64 {
    let required = match task.context.get("required_tools") {
        Some(val) => match serde_json::from_value::<Vec<String>>(val.clone()) {
            Ok(tools) => tools,
            Err(_) => return 1.0, // malformed → no penalty
        },
        None => return 1.0, // no requirement → full score
    };
    if required.is_empty() {
        return 1.0;
    }
    let matched = required.iter().filter(|t| agent.tools.contains(t)).count();
    matched as f64 / required.len() as f64
}

/// Score how well the agent's complexity matches the task's.
fn complexity_score(agent: &AgentDefinition, task: &Task) -> f64 {
    let task_complexity = task
        .context
        .get("complexity")
        .and_then(|v| v.as_str())
        .unwrap_or("medium");
    let agent_level = complexity_level(&agent.complexity) as f64;
    let task_level = complexity_level(task_complexity) as f64;
    1.0 - ((agent_level - task_level).abs() / 3.0)
}

/// Score GPU compatibility.
fn gpu_score(agent: &AgentDefinition, task: &Task) -> f64 {
    let gpu_required = task
        .context
        .get("gpu_required")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !gpu_required {
        return 1.0;
    }
    if agent.gpu_required || agent.gpu_preferred {
        1.0
    } else {
        0.0
    }
}

/// Score domain match.
fn domain_score(agent: &AgentDefinition, task: &Task) -> f64 {
    let task_domain = match task.context.get("domain").and_then(|v| v.as_str()) {
        Some(d) => d,
        None => return 1.0,
    };
    match &agent.domain {
        Some(agent_domain) if agent_domain == task_domain => 1.0,
        Some(_) => 0.0,
        None => 1.0, // agent has no domain constraint → compatible
    }
}

/// Score an agent's suitability for a given task (0.0–1.0).
///
/// Scoring factors:
/// - Tool coverage (0.40): fraction of required tools the agent provides
/// - Complexity alignment (0.30): how well complexity levels match
/// - GPU match (0.15): GPU capability when the task requires it
/// - Domain match (0.15): domain compatibility
pub fn score_agent(agent: &AgentDefinition, task: &Task) -> f64 {
    let tool = tool_coverage_score(agent, task);
    let complexity = complexity_score(agent, task);
    let gpu = gpu_score(agent, task);
    let domain = domain_score(agent, task);

    let score = WEIGHT_TOOL_COVERAGE * tool
        + WEIGHT_COMPLEXITY * complexity
        + WEIGHT_GPU * gpu
        + WEIGHT_DOMAIN * domain;

    score.clamp(0.0, 1.0)
}

/// Rank agents by suitability for a task, returning (index, score) pairs sorted descending.
pub fn rank_agents(agents: &[AgentDefinition], task: &Task) -> Vec<(usize, f64)> {
    let mut scored: Vec<(usize, f64)> = agents
        .iter()
        .enumerate()
        .map(|(i, agent)| (i, score_agent(agent, task)))
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_agent(tools: Vec<&str>, complexity: &str, domain: Option<&str>) -> AgentDefinition {
        AgentDefinition {
            agent_key: "test-agent".into(),
            name: "Test Agent".into(),
            role: "tester".into(),
            goal: "test things".into(),
            backstory: None,
            domain: domain.map(|s| s.to_string()),
            tools: tools.into_iter().map(|s| s.to_string()).collect(),
            complexity: complexity.to_string(),
            llm_model: None,
            gpu_required: false,
            gpu_preferred: false,
            gpu_memory_min_mb: None,
        }
    }

    fn make_task_with_context(ctx: serde_json::Value) -> Task {
        let mut task = Task::new("test task");
        if let serde_json::Value::Object(map) = ctx {
            for (k, v) in map {
                task.context.insert(k, v);
            }
        }
        task
    }

    #[test]
    fn test_perfect_score() {
        let agent = make_agent(vec!["lint", "test"], "medium", Some("quality"));
        let task = make_task_with_context(json!({
            "required_tools": ["lint", "test"],
            "complexity": "medium",
            "domain": "quality"
        }));
        let score = score_agent(&agent, &task);
        assert!(
            (score - 1.0).abs() < f64::EPSILON,
            "expected 1.0, got {score}"
        );
    }

    #[test]
    fn test_no_tools_match() {
        let agent = make_agent(vec!["deploy"], "medium", None);
        let task = make_task_with_context(json!({
            "required_tools": ["lint", "test", "scan"]
        }));
        let score = score_agent(&agent, &task);
        // tool coverage = 0.0, complexity = 1.0, gpu = 1.0, domain = 1.0
        let expected = 0.0 * 0.4 + 1.0 * 0.3 + 1.0 * 0.15 + 1.0 * 0.15;
        assert!(
            (score - expected).abs() < 1e-9,
            "expected {expected}, got {score}"
        );
    }

    #[test]
    fn test_partial_tool_coverage() {
        let agent = make_agent(vec!["lint", "test"], "medium", None);
        let task = make_task_with_context(json!({
            "required_tools": ["lint", "test", "scan", "deploy"]
        }));
        let tool = 2.0 / 4.0; // 0.5
        let expected = tool * 0.4 + 1.0 * 0.3 + 1.0 * 0.15 + 1.0 * 0.15;
        let score = score_agent(&agent, &task);
        assert!(
            (score - expected).abs() < 1e-9,
            "expected {expected}, got {score}"
        );
    }

    #[test]
    fn test_complexity_mismatch() {
        let agent = make_agent(vec![], "low", None);
        let task = make_task_with_context(json!({ "complexity": "high" }));
        // tool = 1.0, complexity = 1 - 2/3 = 1/3, gpu = 1.0, domain = 1.0
        let expected = 1.0 * 0.4 + (1.0 / 3.0) * 0.3 + 1.0 * 0.15 + 1.0 * 0.15;
        let score = score_agent(&agent, &task);
        assert!(
            (score - expected).abs() < 1e-9,
            "expected {expected}, got {score}"
        );
    }

    #[test]
    fn test_gpu_required_agent_has_it() {
        let mut agent = make_agent(vec![], "medium", None);
        agent.gpu_required = true;
        let task = make_task_with_context(json!({ "gpu_required": true }));
        let score = score_agent(&agent, &task);
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_gpu_required_agent_prefers_it() {
        let mut agent = make_agent(vec![], "medium", None);
        agent.gpu_preferred = true;
        let task = make_task_with_context(json!({ "gpu_required": true }));
        let score = score_agent(&agent, &task);
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_gpu_required_agent_lacks_it() {
        let agent = make_agent(vec![], "medium", None);
        let task = make_task_with_context(json!({ "gpu_required": true }));
        // gpu = 0.0
        let expected = 1.0 * 0.4 + 1.0 * 0.3 + 0.0 * 0.15 + 1.0 * 0.15;
        let score = score_agent(&agent, &task);
        assert!(
            (score - expected).abs() < 1e-9,
            "expected {expected}, got {score}"
        );
    }

    #[test]
    fn test_domain_mismatch() {
        let agent = make_agent(vec![], "medium", Some("devops"));
        let task = make_task_with_context(json!({ "domain": "quality" }));
        let expected = 1.0 * 0.4 + 1.0 * 0.3 + 1.0 * 0.15 + 0.0 * 0.15;
        let score = score_agent(&agent, &task);
        assert!(
            (score - expected).abs() < 1e-9,
            "expected {expected}, got {score}"
        );
    }

    #[test]
    fn test_no_context_full_score() {
        let agent = make_agent(vec![], "medium", None);
        let task = Task::new("simple task");
        let score = score_agent(&agent, &task);
        assert!(
            (score - 1.0).abs() < f64::EPSILON,
            "expected 1.0, got {score}"
        );
    }

    #[test]
    fn test_rank_agents_ordering() {
        let good = make_agent(vec!["lint", "test"], "medium", Some("quality"));
        let bad = make_agent(vec![], "high", Some("devops"));
        let task = make_task_with_context(json!({
            "required_tools": ["lint", "test"],
            "complexity": "medium",
            "domain": "quality"
        }));
        let ranked = rank_agents(&[bad.clone(), good.clone()], &task);
        assert_eq!(ranked[0].0, 1, "good agent should be ranked first");
        assert!(ranked[0].1 > ranked[1].1);
    }

    #[test]
    fn test_rank_agents_empty() {
        let task = Task::new("anything");
        let ranked = rank_agents(&[], &task);
        assert!(ranked.is_empty());
    }
}
