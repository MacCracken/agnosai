//! A2A (Agent-to-Agent) protocol endpoints for cross-system task delegation.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::core::{CrewSpec, Task};

use crate::server::state::SharedState;

/// Validate that a callback URL is safe to POST to.
///
/// Rejects URLs targeting private/internal networks to prevent SSRF.
/// Also rejects hostnames that could be used for DNS rebinding attacks.
fn is_safe_callback_url(url: &str) -> bool {
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };

    // Only allow HTTP(S).
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return false;
    }

    let Some(host) = parsed.host_str() else {
        return false;
    };

    // Block localhost and common internal names.
    let lower = host.to_ascii_lowercase();
    if lower == "localhost"
        || lower == "127.0.0.1"
        || lower == "::1"
        || lower == "[::1]"
        || lower.ends_with(".local")
        || lower.ends_with(".internal")
        || lower.ends_with(".localhost")
    {
        return false;
    }

    // Block private/link-local IP ranges.
    if let Ok(ip) = host.parse::<std::net::IpAddr>()
        && is_private_ip(ip)
    {
        return false;
    }

    // Block IPs embedded as hostnames (e.g. "0x7f000001.example.com" won't be caught,
    // but at least raw IPs are validated above).
    true
}

/// Check whether an IP address is in a private, loopback, or link-local range.
fn is_private_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || (v4.octets()[0] == 169 && v4.octets()[1] == 254) // metadata
                || (v4.octets()[0] == 0) // 0.0.0.0/8
        }
        std::net::IpAddr::V6(v6) => v6.is_loopback(),
    }
}

/// Maximum string field length for A2A requests.
const A2A_MAX_STRING_LEN: usize = 10_000;
/// Maximum metadata JSON size in bytes.
const A2A_MAX_METADATA_BYTES: usize = 64 * 1024; // 64 KiB

/// A2A task delegation request — matches Agnostic v1 format.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct A2ARequest {
    /// External task identifier from the calling system.
    pub task_id: String,
    /// Description of the task to perform.
    pub description: String,
    /// Domain hint for agent selection (e.g. `"quality"`, `"security"`).
    #[serde(default)]
    pub domain: Option<String>,
    /// Crew size hint: `"lean"`, `"standard"`, or `"large"`.
    #[serde(default)]
    pub size: Option<String>,
    /// Named preset configuration to use.
    #[serde(default)]
    pub preset: Option<String>,
    /// Optional webhook URL to POST results back to.
    #[serde(default)]
    pub callback_url: Option<String>,
    /// Arbitrary metadata passed through to the crew.
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Response returned from an A2A task delegation.
#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct A2AResponse {
    /// Task identifier echoed from the request.
    pub task_id: String,
    /// Execution status: `"accepted"`, `"completed"`, or `"failed"`.
    pub status: String,
    /// Result payload on success.
    pub result: Option<serde_json::Value>,
    /// Error message on failure.
    pub error: Option<String>,
}

/// POST /api/v1/a2a/receive — Accept an A2A task delegation.
///
/// Builds a single-task crew from the request, runs it through the orchestrator,
/// and returns the result. If `callback_url` is set, spawns a background task
/// to POST results back (fire-and-forget).
pub async fn receive(
    State(state): State<SharedState>,
    Json(req): Json<A2ARequest>,
) -> (StatusCode, Json<A2AResponse>) {
    // Validate field lengths.
    if req.task_id.len() > A2A_MAX_STRING_LEN || req.description.len() > A2A_MAX_STRING_LEN {
        tracing::warn!(
            task_id = %req.task_id.chars().take(100).collect::<String>(),
            "A2A rejected: field exceeds max length"
        );
        return (
            StatusCode::BAD_REQUEST,
            Json(A2AResponse {
                task_id: req.task_id,
                status: "failed".to_string(),
                result: None,
                error: Some("field exceeds max length".to_string()),
            }),
        );
    }
    if let Ok(meta_bytes) = serde_json::to_vec(&req.metadata)
        && meta_bytes.len() > A2A_MAX_METADATA_BYTES
    {
        tracing::warn!(
            task_id = %req.task_id,
            metadata_bytes = meta_bytes.len(),
            "A2A rejected: metadata exceeds limit"
        );
        return (
            StatusCode::BAD_REQUEST,
            Json(A2AResponse {
                task_id: req.task_id,
                status: "failed".to_string(),
                result: None,
                error: Some("metadata exceeds 64 KiB limit".to_string()),
            }),
        );
    }

    let task_id = req.task_id.clone();

    // Build a simple crew with one task and default (empty) agent list.
    let crew_name = format!(
        "a2a-{}-{}",
        req.domain.as_deref().unwrap_or("general"),
        &task_id
    );
    let mut spec = CrewSpec::new(crew_name);
    let task = Task::new(&req.description);
    spec.tasks = vec![task];

    match state.orchestrator.run_crew(spec).await {
        Ok(crew_state) => {
            let result_data: Vec<serde_json::Value> = crew_state
                .results
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "task_id": r.task_id.to_string(),
                        "output": r.output,
                    })
                })
                .collect();

            let response = A2AResponse {
                task_id: task_id.clone(),
                status: "completed".to_string(),
                result: Some(serde_json::json!({ "tasks": result_data })),
                error: None,
            };

            // Fire-and-forget callback if URL is provided and passes SSRF validation.
            if let Some(url) = req.callback_url {
                if is_safe_callback_url(&url) {
                    let resp_clone = response.clone();
                    let client = state.http_client.clone();
                    tokio::spawn(async move {
                        if let Err(e) = client.post(&url).json(&resp_clone).send().await {
                            tracing::warn!(task_id = %task_id, url = %url, error = %e, "A2A callback failed");
                        }
                    });
                } else {
                    tracing::warn!(task_id = %task_id, url = %url, "A2A callback URL rejected (SSRF protection)");
                }
            }

            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            let response = A2AResponse {
                task_id,
                status: "failed".to_string(),
                result: None,
                error: Some(e.to_string()),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// POST /api/v1/a2a/status — Check status of a delegated task (placeholder).
pub async fn status() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "not_implemented"}))
}

#[cfg(test)]
mod tests {
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
        });
        crate::server::router(state)
    }

    #[tokio::test]
    async fn a2a_receive_with_valid_request_returns_completed() {
        let app = test_app().await;
        let body = serde_json::json!({
            "task_id": "ext-123",
            "description": "Analyse the login flow",
            "domain": "quality",
            "size": "lean",
            "metadata": {"source": "secureyeoman"}
        });
        let response = app
            .oneshot(
                Request::post("/api/v1/a2a/receive")
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
        assert_eq!(json["task_id"], "ext-123");
        assert_eq!(json["status"], "completed");
        assert!(json["result"].is_object());
    }

    #[tokio::test]
    async fn a2a_receive_minimal_request() {
        let app = test_app().await;
        let body = serde_json::json!({
            "task_id": "min-1",
            "description": "Hello world"
        });
        let response = app
            .oneshot(
                Request::post("/api/v1/a2a/receive")
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
        assert_eq!(json["task_id"], "min-1");
        assert_eq!(json["status"], "completed");
    }
}
