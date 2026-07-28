//! Hierarchical process mode — manager-driven task delegation.
//!
//! In hierarchical mode, a designated manager agent delegates tasks to worker
//! agents based on scoring. The manager does not execute tasks directly; it
//! assigns each task to the best-scoring agent from the available pool.

use crate::core::agent::AgentDefinition;
use crate::core::task::{Task, TaskId};
use crate::orchestrator::scoring;

/// A manager agent that delegates tasks to worker agents.
///
/// The manager holds references to the crew's agent pool and task list,
/// and uses scoring to assign tasks to the best-fit agents.
#[derive(Debug)]
#[non_exhaustive]
pub struct ManagerAgent<'a> {
    /// The manager agent's own definition.
    pub definition: &'a AgentDefinition,
    /// Available worker agents (excludes the manager).
    pub agents: &'a [AgentDefinition],
    /// Tasks to be delegated.
    pub tasks: &'a [Task],
}

impl<'a> ManagerAgent<'a> {
    /// Create a new manager agent.
    pub fn new(
        definition: &'a AgentDefinition,
        agents: &'a [AgentDefinition],
        tasks: &'a [Task],
    ) -> Self {
        Self {
            definition,
            agents,
            tasks,
        }
    }

    /// Delegate all tasks to the best-scoring agents.
    ///
    /// Returns a list of `(TaskId, agent_key)` assignments. Each task is
    /// assigned to the highest-scoring agent. If no agents are available,
    /// tasks are assigned to the manager as a fallback.
    #[must_use]
    pub fn delegate(&self) -> Vec<(TaskId, String)> {
        delegate_tasks(self.definition, self.agents, self.tasks)
    }
}

/// Assign tasks to agents based on scoring.
///
/// For each task, ranks all available agents using [`scoring::rank_agents`]
/// and picks the highest-scoring one. If no workers are available, the
/// manager is used as fallback.
///
/// # Returns
///
/// A vec of `(TaskId, agent_key)` pairs indicating which agent should
/// execute which task.
#[must_use]
#[tracing::instrument(skip_all, fields(manager_key = %manager.agent_key, num_agents = agents.len(), num_tasks = tasks.len()))]
pub fn delegate_tasks(
    manager: &AgentDefinition,
    agents: &[AgentDefinition],
    tasks: &[Task],
) -> Vec<(TaskId, String)> {
    let mut assignments = Vec::with_capacity(tasks.len());

    for task in tasks {
        if agents.is_empty() {
            // No workers available — manager takes the task.
            tracing::warn!(
                task_id = %task.id,
                manager_key = %manager.agent_key,
                "no workers available, manager takes task"
            );
            assignments.push((task.id, manager.agent_key.clone()));
            continue;
        }

        let ranked = scoring::rank_agents(agents, task);

        let (best_idx, best_score) = ranked.first().copied().unwrap_or((0, 0.0));

        let agent_key = &agents[best_idx].agent_key;

        tracing::debug!(
            task_id = %task.id,
            agent_key = %agent_key,
            score = best_score,
            "delegated task to agent"
        );

        assignments.push((task.id, agent_key.clone()));
    }

    tracing::info!(
        assignments = assignments.len(),
        "hierarchical delegation complete"
    );

    assignments
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::AgentDefinition;
    use crate::core::task::Task;
    use serde_json::json;

    fn make_agent(
        key: &str,
        tools: Vec<&str>,
        complexity: &str,
        domain: Option<&str>,
    ) -> AgentDefinition {
        AgentDefinition {
            agent_key: key.into(),
            name: format!("Agent {key}"),
            role: "worker".into(),
            goal: "do work".into(),
            backstory: None,
            domain: domain.map(|s| s.to_string()),
            tools: tools.into_iter().map(|s| s.to_string()).collect(),
            complexity: complexity.to_string(),
            llm_model: None,
            gpu_required: false,
            gpu_preferred: false,
            gpu_memory_min_mb: None,
            hardware: None,
            personality: None,
        }
    }

    fn make_task_with_context(desc: &str, ctx: serde_json::Value) -> Task {
        let mut task = Task::new(desc);
        if let serde_json::Value::Object(map) = ctx {
            for (k, v) in map {
                task.context.insert(k, v);
            }
        }
        task
    }

    #[test]
    fn delegate_assigns_best_agent() {
        let manager = make_agent("manager", vec![], "medium", None);
        let coder = make_agent("coder", vec!["lint", "test"], "medium", Some("quality"));
        let deployer = make_agent("deployer", vec!["deploy"], "high", Some("devops"));

        let task = make_task_with_context(
            "code review",
            json!({
                "required_tools": ["lint", "test"],
                "domain": "quality",
                "complexity": "medium"
            }),
        );

        let assignments = delegate_tasks(&manager, &[coder.clone(), deployer], &[task]);
        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0].1, "coder");
    }

    #[test]
    fn delegate_multiple_tasks() {
        let manager = make_agent("manager", vec![], "medium", None);
        let coder = make_agent("coder", vec!["lint", "test"], "medium", Some("quality"));
        let deployer = make_agent("deployer", vec!["deploy"], "high", Some("devops"));

        let code_task = make_task_with_context(
            "code review",
            json!({
                "required_tools": ["lint", "test"],
                "domain": "quality"
            }),
        );
        let deploy_task = make_task_with_context(
            "deploy app",
            json!({
                "required_tools": ["deploy"],
                "domain": "devops",
                "complexity": "high"
            }),
        );

        let assignments = delegate_tasks(
            &manager,
            &[coder.clone(), deployer.clone()],
            &[code_task, deploy_task],
        );
        assert_eq!(assignments.len(), 2);
        assert_eq!(assignments[0].1, "coder");
        assert_eq!(assignments[1].1, "deployer");
    }

    #[test]
    fn delegate_no_workers_falls_back_to_manager() {
        let manager = make_agent("manager", vec![], "medium", None);
        let task = Task::new("orphan task");
        let assignments = delegate_tasks(&manager, &[], &[task]);
        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0].1, "manager");
    }

    #[test]
    fn delegate_empty_tasks() {
        let manager = make_agent("manager", vec![], "medium", None);
        let worker = make_agent("worker", vec![], "medium", None);
        let assignments = delegate_tasks(&manager, &[worker], &[]);
        assert!(assignments.is_empty());
    }

    #[test]
    fn manager_agent_delegate_method() {
        let manager_def = make_agent("manager", vec![], "medium", None);
        let worker = make_agent("worker", vec!["lint"], "medium", None);
        let task = Task::new("some task");

        let agents = vec![worker];
        let tasks = vec![task];
        let mgr = ManagerAgent::new(&manager_def, &agents, &tasks);

        let assignments = mgr.delegate();
        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0].1, "worker");
    }

    #[test]
    fn delegate_preserves_task_ids() {
        let manager = make_agent("manager", vec![], "medium", None);
        let worker = make_agent("worker", vec![], "medium", None);
        let t1 = Task::new("task 1");
        let t2 = Task::new("task 2");
        let id1 = t1.id;
        let id2 = t2.id;

        let assignments = delegate_tasks(&manager, &[worker], &[t1, t2]);
        assert_eq!(assignments[0].0, id1);
        assert_eq!(assignments[1].0, id2);
    }
}
