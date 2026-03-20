use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::{AgentDefinition, CrewSpec, ProcessMode, Task, TaskPriority};

use crate::server::state::SharedState;

#[derive(Debug, Deserialize)]
pub struct TaskRequest {
    pub description: String,
    #[serde(default)]
    pub expected_output: Option<String>,
    #[serde(default)]
    pub priority: Option<TaskPriority>,
    #[serde(default)]
    pub dependencies: Vec<usize>,
}

#[derive(Debug, Deserialize)]
pub struct CrewRunRequest {
    pub name: String,
    pub agents: Vec<AgentDefinition>,
    pub tasks: Vec<TaskRequest>,
    #[serde(default)]
    pub process: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CrewRunResponse {
    pub crew_id: Uuid,
    pub status: String,
    pub results: Vec<TaskResultResponse>,
}

#[derive(Debug, Serialize)]
pub struct TaskResultResponse {
    pub task_id: Uuid,
    pub output: String,
    pub status: String,
}

/// Maximum number of agents per crew.
const MAX_AGENTS: usize = 100;
/// Maximum number of tasks per crew.
const MAX_TASKS: usize = 1000;
/// Maximum string field length.
const MAX_STRING_LEN: usize = 10_000;

fn validate_crew_request(req: &CrewRunRequest) -> Result<(), String> {
    if req.name.is_empty() || req.name.len() > MAX_STRING_LEN {
        return Err(format!("name must be 1-{MAX_STRING_LEN} characters"));
    }
    if req.agents.is_empty() {
        return Err("at least one agent is required".into());
    }
    if req.agents.len() > MAX_AGENTS {
        return Err(format!("at most {MAX_AGENTS} agents allowed"));
    }
    if req.tasks.is_empty() {
        return Err("at least one task is required".into());
    }
    if req.tasks.len() > MAX_TASKS {
        return Err(format!("at most {MAX_TASKS} tasks allowed"));
    }
    for (i, agent) in req.agents.iter().enumerate() {
        if agent.agent_key.len() > MAX_STRING_LEN || agent.role.len() > MAX_STRING_LEN {
            return Err(format!("agent {i}: field exceeds max length"));
        }
    }
    for (i, task) in req.tasks.iter().enumerate() {
        if task.description.len() > MAX_STRING_LEN {
            return Err(format!("task {i}: description exceeds max length"));
        }
        for &dep in &task.dependencies {
            if dep >= req.tasks.len() {
                return Err(format!("task {i}: dependency index {dep} out of range"));
            }
        }
    }
    Ok(())
}

pub async fn create_crew(
    State(state): State<SharedState>,
    Json(req): Json<CrewRunRequest>,
) -> Result<Json<CrewRunResponse>, (StatusCode, Json<serde_json::Value>)> {
    if let Err(msg) = validate_crew_request(&req) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": msg})),
        ));
    }
    let process = match req.process.as_deref() {
        Some("hierarchical") => {
            // Use first agent as manager placeholder
            ProcessMode::Hierarchical {
                manager: Uuid::new_v4(),
            }
        }
        Some("dag") => ProcessMode::Dag,
        Some("parallel") => ProcessMode::Parallel { max_concurrency: 4 },
        _ => ProcessMode::Sequential,
    };

    let mut spec = CrewSpec::new(req.name);
    spec.agents = req.agents;
    spec.process = process;

    // Build tasks, tracking their UUIDs for dependency mapping.
    let mut tasks: Vec<Task> = Vec::with_capacity(req.tasks.len());
    for task_req in &req.tasks {
        let mut task = Task::new(&task_req.description);
        task.expected_output = task_req.expected_output.clone();
        if let Some(priority) = task_req.priority {
            task.priority = priority;
        }
        tasks.push(task);
    }

    // Resolve index-based dependencies to UUIDs.
    // Collect IDs first to avoid simultaneous borrow.
    let task_ids: Vec<_> = tasks.iter().map(|t| t.id).collect();
    for (i, task_req) in req.tasks.iter().enumerate() {
        for &dep_idx in &task_req.dependencies {
            if dep_idx < task_ids.len() {
                tasks[i].dependencies.push(task_ids[dep_idx]);
            }
        }
    }

    spec.tasks = tasks;

    match state.orchestrator.run_crew(spec).await {
        Ok(crew_state) => {
            let results = crew_state
                .results
                .iter()
                .map(|r| TaskResultResponse {
                    task_id: r.task_id,
                    output: r.output.clone(),
                    status: serde_json::to_value(r.status)
                        .ok()
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_else(|| "unknown".to_string()),
                })
                .collect();

            let status = serde_json::to_value(crew_state.status)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_else(|| "unknown".to_string());

            Ok(Json(CrewRunResponse {
                crew_id: crew_state.crew_id,
                status,
                results,
            }))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )),
    }
}

pub async fn get_crew(
    axum::extract::Path(_id): axum::extract::Path<Uuid>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Placeholder — full state tracking is future work.
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({"error": "crew not found"})),
    )
}

#[cfg(test)]
mod tests {
    use crate::server::state::{AppState, SharedState};
    use crate::orchestrator::Orchestrator;
    use crate::tools::ToolRegistry;
    use axum::Router;
    use axum::http::{Request, StatusCode};
    use std::sync::Arc;
    use tower::ServiceExt;

    async fn test_app() -> Router {
        let orchestrator = Orchestrator::new(Default::default()).await.unwrap();
        let tools = Arc::new(ToolRegistry::new());
        let state: SharedState = Arc::new(AppState {
            orchestrator,
            tools,
        });
        crate::server::router(state)
    }

    #[tokio::test]
    async fn post_crews_with_valid_body_returns_result() {
        let app = test_app().await;
        let body = serde_json::json!({
            "name": "test-crew",
            "agents": [{
                "agent_key": "tester",
                "name": "Tester",
                "role": "tester",
                "goal": "test things"
            }],
            "tasks": [{
                "description": "Run tests",
                "expected_output": "test results"
            }]
        });
        let response = app
            .oneshot(
                Request::post("/api/v1/crews")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(json["crew_id"].is_string());
        assert!(json["status"].is_string());
    }

    #[tokio::test]
    async fn post_crews_with_empty_agents_returns_bad_request() {
        let app = test_app().await;
        let body = serde_json::json!({
            "name": "empty-crew",
            "agents": [],
            "tasks": []
        });
        let response = app
            .oneshot(
                Request::post("/api/v1/crews")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(json["error"].as_str().unwrap().contains("agent"));
    }
}
