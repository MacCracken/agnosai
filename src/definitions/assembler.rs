//! Crew assembly from team specs — matches roles to agent definitions.

use crate::core::agent::AgentDefinition;
use crate::core::crew::CrewSpec;
use crate::core::task::Task;

/// A requested team member specification.
#[derive(Debug, Clone)]
pub struct TeamMember {
    pub role: String,
    pub tools: Vec<String>,
    pub complexity: Option<String>,
}

/// Assemble a crew from a list of role descriptions, matching against available agent definitions.
/// For each member, selects the best-matching agent definition from `available`.
pub fn assemble_team(
    members: &[TeamMember],
    available: &[AgentDefinition],
) -> Vec<AgentDefinition> {
    members
        .iter()
        .filter_map(|member| {
            available
                .iter()
                .max_by(|a, b| {
                    match_score(member, a)
                        .partial_cmp(&match_score(member, b))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .filter(|agent| match_score(member, agent) > 0.0)
                .cloned()
        })
        .collect()
}

/// Score how well an agent matches a team member spec.
/// Higher score = better match.
fn match_score(member: &TeamMember, agent: &AgentDefinition) -> f64 {
    let mut score = 0.0;

    // Role matching: check if the agent's role contains the requested role (case-insensitive).
    let member_role = member.role.to_lowercase();
    let agent_role = agent.role.to_lowercase();
    if agent_role == member_role {
        score += 10.0;
    } else if agent_role.contains(&member_role) || member_role.contains(&agent_role) {
        score += 5.0;
    }

    // Also check agent name and goal for role keywords.
    let agent_name = agent.name.to_lowercase();
    if agent_name.contains(&member_role) {
        score += 2.0;
    }

    // Tool matching: count overlapping tools.
    for tool in &member.tools {
        let tool_lower = tool.to_lowercase();
        if agent.tools.iter().any(|t| t.to_lowercase() == tool_lower) {
            score += 3.0;
        }
    }

    // Complexity matching.
    if let Some(ref requested) = member.complexity
        && agent.complexity.to_lowercase() == requested.to_lowercase()
    {
        score += 2.0;
    }

    score
}

/// Build a complete `CrewSpec` from members, available definitions, and tasks.
pub fn build_crew(
    name: &str,
    members: &[TeamMember],
    available: &[AgentDefinition],
    tasks: Vec<Task>,
) -> CrewSpec {
    let agents = assemble_team(members, available);
    let mut crew = CrewSpec::new(name);
    crew.agents = agents;
    crew.tasks = tasks;
    crew
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_agent(key: &str, role: &str, tools: Vec<&str>, complexity: &str) -> AgentDefinition {
        AgentDefinition {
            agent_key: key.to_string(),
            name: format!("{key} Agent"),
            role: role.to_string(),
            goal: format!("Handle {role} tasks"),
            backstory: None,
            domain: None,
            tools: tools.into_iter().map(String::from).collect(),
            complexity: complexity.to_string(),
            llm_model: None,
            gpu_required: false,
            gpu_preferred: false,
            gpu_memory_min_mb: None,
            hardware: None,
        }
    }

    #[test]
    fn match_by_role() {
        let agents = vec![
            make_agent("qa", "tester", vec![], "medium"),
            make_agent("dev", "developer", vec![], "medium"),
        ];
        let members = vec![TeamMember {
            role: "tester".to_string(),
            tools: vec![],
            complexity: None,
        }];

        let team = assemble_team(&members, &agents);
        assert_eq!(team.len(), 1);
        assert_eq!(team[0].agent_key, "qa");
    }

    #[test]
    fn match_by_tools() {
        let agents = vec![
            make_agent("a1", "worker", vec!["selenium", "pytest"], "medium"),
            make_agent("a2", "worker", vec!["docker", "kubectl"], "medium"),
        ];
        let members = vec![TeamMember {
            role: "worker".to_string(),
            tools: vec!["selenium".to_string()],
            complexity: None,
        }];

        let team = assemble_team(&members, &agents);
        assert_eq!(team.len(), 1);
        assert_eq!(team[0].agent_key, "a1");
    }

    #[test]
    fn match_by_complexity() {
        let agents = vec![
            make_agent("simple", "coder", vec![], "low"),
            make_agent("complex", "coder", vec![], "high"),
        ];
        let members = vec![TeamMember {
            role: "coder".to_string(),
            tools: vec![],
            complexity: Some("high".to_string()),
        }];

        let team = assemble_team(&members, &agents);
        assert_eq!(team.len(), 1);
        assert_eq!(team[0].agent_key, "complex");
    }

    #[test]
    fn partial_role_match() {
        let agents = vec![make_agent("lead", "lead tester", vec![], "medium")];
        let members = vec![TeamMember {
            role: "tester".to_string(),
            tools: vec![],
            complexity: None,
        }];

        let team = assemble_team(&members, &agents);
        assert_eq!(team.len(), 1);
        assert_eq!(team[0].agent_key, "lead");
    }

    #[test]
    fn no_match_returns_empty() {
        let agents = vec![make_agent("dev", "developer", vec![], "medium")];
        let members = vec![TeamMember {
            role: "designer".to_string(),
            tools: vec!["figma".to_string()],
            complexity: None,
        }];

        let team = assemble_team(&members, &agents);
        assert!(team.is_empty());
    }

    #[test]
    fn build_complete_crew() {
        let agents = vec![
            make_agent("qa", "tester", vec!["selenium"], "medium"),
            make_agent("dev", "developer", vec!["git"], "high"),
        ];
        let members = vec![
            TeamMember {
                role: "tester".to_string(),
                tools: vec![],
                complexity: None,
            },
            TeamMember {
                role: "developer".to_string(),
                tools: vec![],
                complexity: None,
            },
        ];
        let tasks = vec![Task::new("Run tests"), Task::new("Fix bugs")];

        let crew = build_crew("test-crew", &members, &agents, tasks);
        assert_eq!(crew.name, "test-crew");
        assert_eq!(crew.agents.len(), 2);
        assert_eq!(crew.tasks.len(), 2);
    }

    #[test]
    fn build_crew_with_empty_members() {
        let crew = build_crew("empty", &[], &[], vec![]);
        assert_eq!(crew.name, "empty");
        assert!(crew.agents.is_empty());
        assert!(crew.tasks.is_empty());
    }
}
