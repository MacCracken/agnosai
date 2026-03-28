//! Dashboard API — crew history and agent performance summaries.

use axum::Json;
use axum::extract::State;
use serde::Serialize;

use crate::server::state::SharedState;

/// Summary of a crew execution for the dashboard.
#[derive(Debug, Serialize)]
#[non_exhaustive]
pub struct CrewSummary {
    pub crew_id: String,
    pub status: String,
    pub task_count: usize,
    pub wall_ms: u64,
    pub cost_usd: f64,
}

/// Agent performance summary for the dashboard.
#[derive(Debug, Serialize)]
#[non_exhaustive]
pub struct AgentSummary {
    pub agent_key: String,
    pub name: String,
    pub role: String,
    pub tool_count: usize,
}

/// GET /api/v1/dashboard/crews — List recent crew execution summaries.
pub async fn crew_history(State(state): State<SharedState>) -> Json<Vec<CrewSummary>> {
    let orch_state = state.orchestrator.state().read().await;
    let summaries: Vec<CrewSummary> = orch_state
        .active_crews
        .values()
        .map(|c| CrewSummary {
            crew_id: c.crew_id.to_string(),
            status: format!("{:?}", c.status),
            task_count: c.results.len(),
            wall_ms: c.profile.as_ref().map_or(0, |p| p.wall_ms),
            cost_usd: c.profile.as_ref().map_or(0.0, |p| p.cost_usd),
        })
        .collect();
    Json(summaries)
}

/// GET /api/v1/dashboard/agents — List agent definitions with summary info.
pub async fn agent_performance(State(state): State<SharedState>) -> Json<Vec<AgentSummary>> {
    // Collect unique agents from recent crew runs.
    let orch_state = state.orchestrator.state().read().await;
    let mut seen = std::collections::HashSet::new();
    let mut summaries = Vec::new();

    for crew_state in orch_state.active_crews.values() {
        for result in &crew_state.results {
            if let Some(agent_key) = result.metadata.get("agent").and_then(|v| v.as_str())
                && seen.insert(agent_key.to_string())
            {
                summaries.push(AgentSummary {
                    agent_key: agent_key.to_string(),
                    name: result
                        .metadata
                        .get("agent_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or(agent_key)
                        .to_string(),
                    role: result
                        .metadata
                        .get("agent_role")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    tool_count: 0,
                });
            }
        }
    }

    Json(summaries)
}
