use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::crew::CrewProfile;
use crate::core::{AgentDefinition, CrewSpec, ProcessMode, Task, TaskPriority};

use crate::server::state::SharedState;

/// Inbound task definition within a crew creation request.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct TaskRequest {
    /// Description of the task to perform.
    pub description: String,
    /// Optional expected output description.
    #[serde(default)]
    pub expected_output: Option<String>,
    /// Optional priority override.
    #[serde(default)]
    pub priority: Option<TaskPriority>,
    /// Index-based dependency list referencing other tasks in the request.
    #[serde(default)]
    pub dependencies: Vec<usize>,
}

/// Request body for creating and running a new crew.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct CrewRunRequest {
    /// Name for the crew.
    pub name: String,
    /// Agent definitions to include in the crew.
    pub agents: Vec<AgentDefinition>,
    /// Tasks for the crew to execute.
    pub tasks: Vec<TaskRequest>,
    /// Optional process mode string (`"sequential"`, `"parallel"`, `"dag"`, `"hierarchical"`).
    #[serde(default)]
    pub process: Option<String>,
    /// Maximum concurrency for parallel mode (default: 4, max: 64).
    #[serde(default)]
    pub max_concurrency: Option<usize>,
}

/// Response body returned after a crew run completes.
#[derive(Debug, Serialize)]
#[non_exhaustive]
pub struct CrewRunResponse {
    /// Unique identifier of the crew that was executed.
    pub crew_id: Uuid,
    /// Overall crew status (e.g. `"completed"`, `"failed"`).
    pub status: String,
    /// Per-task results.
    pub results: Vec<TaskResultResponse>,
    /// Execution profile (timing, cost).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<CrewProfile>,
}

/// Single task result within a crew run response.
#[derive(Debug, Serialize)]
#[non_exhaustive]
pub struct TaskResultResponse {
    /// ID of the completed task.
    pub task_id: Uuid,
    /// Task output text.
    pub output: String,
    /// Task status string.
    pub status: String,
}

/// Maximum number of agents per crew.
const MAX_AGENTS: usize = 100;
/// Maximum number of tasks per crew.
const MAX_TASKS: usize = 1000;
/// Maximum string field length.
const MAX_STRING_LEN: usize = 10_000;

pub(crate) fn validate_crew_request(req: &CrewRunRequest) -> Result<(), String> {
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
            if dep == i {
                return Err(format!("task {i}: self-dependency"));
            }
        }
    }
    // Simple cycle detection via DFS on index-based graph.
    if has_dependency_cycle(req.tasks.len(), &req.tasks) {
        return Err("task dependencies contain a cycle".into());
    }
    Ok(())
}

/// DFS-based cycle detection on index-based task dependencies.
pub(crate) fn has_dependency_cycle(n: usize, tasks: &[TaskRequest]) -> bool {
    // 0 = unvisited, 1 = in-progress, 2 = done
    let mut state = vec![0u8; n];

    fn visit(node: usize, tasks: &[TaskRequest], state: &mut [u8]) -> bool {
        if state[node] == 1 {
            return true; // cycle
        }
        if state[node] == 2 {
            return false;
        }
        state[node] = 1;
        for &dep in &tasks[node].dependencies {
            if dep < tasks.len() && visit(dep, tasks, state) {
                return true;
            }
        }
        state[node] = 2;
        false
    }

    (0..n).any(|i| visit(i, tasks, &mut state))
}

/// POST /api/v1/crews — Create and run a new crew.
#[tracing::instrument(skip(state, req), fields(crew_name = %req.name, agents = req.agents.len(), tasks = req.tasks.len()))]
pub async fn create_crew(
    State(state): State<SharedState>,
    Json(req): Json<CrewRunRequest>,
) -> Result<Json<CrewRunResponse>, (StatusCode, Json<serde_json::Value>)> {
    if let Err(msg) = validate_crew_request(&req) {
        tracing::warn!(crew = %req.name, error = %msg, "crew request validation failed");
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
        Some("parallel") => ProcessMode::Parallel {
            max_concurrency: req.max_concurrency.unwrap_or(4).clamp(1, 64),
        },
        _ => ProcessMode::Sequential,
    };

    let crew_name = req.name.clone();
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
                profile: crew_state.profile,
            }))
        }
        Err(e) => {
            tracing::error!(error = %e, crew = %crew_name, "crew execution failed");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "crew execution failed"})),
            ))
        }
    }
}

/// GET /api/v1/crews/:id — Retrieve crew state.
pub async fn get_crew(
    State(state): State<SharedState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let inner = state.orchestrator.state().read().await;
    if let Some(crew) = inner.active_crews.get(&id) {
        Ok(Json(serde_json::to_value(crew).unwrap_or_default()))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "crew not found"})),
        ))
    }
}

/// POST /api/v1/crews/:id/cancel — Cancel a running crew.
#[tracing::instrument(skip(state), fields(%id))]
pub async fn cancel_crew(
    State(state): State<SharedState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state.orchestrator.cancel_crew(id).await {
        Ok(()) => Ok(Json(
            serde_json::json!({"status": "cancelled", "crew_id": id.to_string()}),
        )),
        Err(_) => Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "crew not found"})),
        )),
    }
}

#[cfg(test)]
mod tests {
    use crate::llm::AuditChain;
    use crate::orchestrator::Orchestrator;
    use crate::server::state::{AppState, SharedState};
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
            auth: Default::default(),
            events: crate::server::sse::EventBus::new(),
            http_client: reqwest::Client::new(),
            audit: Arc::new(AuditChain::new(b"test-key", 1_000)),
            approval_gate: Default::default(),
            definitions: dashmap::DashMap::new(),
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

    // --- Validation unit tests ---

    fn make_agent() -> crate::core::AgentDefinition {
        serde_json::from_value(serde_json::json!({
            "agent_key": "a",
            "name": "Agent",
            "role": "role",
            "goal": "goal"
        }))
        .unwrap()
    }

    fn make_task(desc: &str) -> super::TaskRequest {
        super::TaskRequest {
            description: desc.into(),
            expected_output: None,
            priority: None,
            dependencies: vec![],
        }
    }

    fn make_task_with_deps(desc: &str, deps: Vec<usize>) -> super::TaskRequest {
        super::TaskRequest {
            description: desc.into(),
            expected_output: None,
            priority: None,
            dependencies: deps,
        }
    }

    #[test]
    fn validate_rejects_empty_name() {
        let req = super::CrewRunRequest {
            name: String::new(),
            agents: vec![make_agent()],
            tasks: vec![make_task("do stuff")],
            process: None,
            max_concurrency: None,
        };
        let err = super::validate_crew_request(&req).unwrap_err();
        assert!(err.contains("name"));
    }

    #[test]
    fn validate_rejects_missing_tasks() {
        let req = super::CrewRunRequest {
            name: "crew".into(),
            agents: vec![make_agent()],
            tasks: vec![],
            process: None,
            max_concurrency: None,
        };
        let err = super::validate_crew_request(&req).unwrap_err();
        assert!(err.contains("task"));
    }

    #[test]
    fn validate_rejects_self_dependency() {
        let req = super::CrewRunRequest {
            name: "crew".into(),
            agents: vec![make_agent()],
            tasks: vec![make_task_with_deps("task0", vec![0])],
            process: None,
            max_concurrency: None,
        };
        let err = super::validate_crew_request(&req).unwrap_err();
        assert!(err.contains("self-dependency"));
    }

    #[test]
    fn validate_rejects_out_of_range_dependency() {
        let req = super::CrewRunRequest {
            name: "crew".into(),
            agents: vec![make_agent()],
            tasks: vec![make_task("task0"), make_task_with_deps("task1", vec![99])],
            process: None,
            max_concurrency: None,
        };
        let err = super::validate_crew_request(&req).unwrap_err();
        assert!(err.contains("out of range"));
    }

    #[test]
    fn validate_rejects_dependency_cycle() {
        let req = super::CrewRunRequest {
            name: "crew".into(),
            agents: vec![make_agent()],
            tasks: vec![
                make_task_with_deps("task0", vec![1]),
                make_task_with_deps("task1", vec![0]),
            ],
            process: None,
            max_concurrency: None,
        };
        let err = super::validate_crew_request(&req).unwrap_err();
        assert!(err.contains("cycle"));
    }

    #[test]
    fn validate_accepts_valid_dag() {
        let req = super::CrewRunRequest {
            name: "crew".into(),
            agents: vec![make_agent()],
            tasks: vec![
                make_task("task0"),
                make_task_with_deps("task1", vec![0]),
                make_task_with_deps("task2", vec![0, 1]),
            ],
            process: None,
            max_concurrency: None,
        };
        assert!(super::validate_crew_request(&req).is_ok());
    }

    #[tokio::test]
    async fn get_crew_returns_not_found() {
        let app = test_app().await;
        let id = uuid::Uuid::new_v4();
        let response = app
            .oneshot(
                Request::get(format!("/api/v1/crews/{id}"))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(json["error"].as_str().unwrap().contains("not found"));
    }

    #[tokio::test]
    async fn post_crews_dag_mode() {
        let app = test_app().await;
        let body = serde_json::json!({
            "name": "dag-crew",
            "agents": [{
                "agent_key": "worker",
                "name": "Worker",
                "role": "worker",
                "goal": "do work"
            }],
            "tasks": [
                {"description": "first task"},
                {"description": "second task", "dependencies": [0]}
            ],
            "process": "dag"
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
    }
}
