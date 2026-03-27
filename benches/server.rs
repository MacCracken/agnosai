//! Benchmarks for HTTP request/response cycles through the axum server.
//!
//! Each benchmark uses `tower::ServiceExt::oneshot` to drive a full
//! request through the router stack (middleware, extractors, handler)
//! without binding a TCP socket.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::Request;
use criterion::{Criterion, criterion_group, criterion_main};
use serde_json::json;
use tower::ServiceExt;

use agnosai::llm::AuditChain;
use agnosai::orchestrator::Orchestrator;
use agnosai::server::state::{AppState, SharedState};
use agnosai::server::{self, AuthConfig};
use agnosai::tools::ToolRegistry;
use agnosai::tools::builtin::echo::EchoTool;
use agnosai::tools::builtin::json_transform::JsonTransformTool;

/// Build a test app with echo + json_transform tools registered.
async fn test_app() -> Router {
    let orchestrator = Orchestrator::new(Default::default()).await.unwrap();
    let tools = Arc::new(ToolRegistry::new());
    tools.register(Arc::new(EchoTool));
    tools.register(Arc::new(JsonTransformTool));
    let state: SharedState = Arc::new(AppState {
        orchestrator,
        tools,
        auth: AuthConfig::default(),
        events: agnosai::server::sse::EventBus::new(),
        http_client: reqwest::Client::new(),
        audit: Arc::new(AuditChain::new(b"bench-key", 1_000)),
        approval_gate: Default::default(),
    });
    server::router(state)
}

// ── GET /health ─────────────────────────────────────────────────────

fn bench_get_health(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let app = rt.block_on(test_app());

    c.bench_function("GET /health", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                let app = app.clone();
                async move {
                    let resp = app
                        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
                        .await
                        .unwrap();
                    assert_eq!(resp.status(), 200);
                }
            });
    });
}

// ── GET /ready ──────────────────────────────────────────────────────

fn bench_get_ready(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let app = rt.block_on(test_app());

    c.bench_function("GET /ready", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                let app = app.clone();
                async move {
                    let resp = app
                        .oneshot(Request::get("/ready").body(Body::empty()).unwrap())
                        .await
                        .unwrap();
                    assert_eq!(resp.status(), 200);
                }
            });
    });
}

// ── GET /metrics ────────────────────────────────────────────────────

fn bench_get_metrics(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let app = rt.block_on(test_app());

    c.bench_function("GET /metrics", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                let app = app.clone();
                async move {
                    let resp = app
                        .oneshot(Request::get("/metrics").body(Body::empty()).unwrap())
                        .await
                        .unwrap();
                    assert_eq!(resp.status(), 200);
                }
            });
    });
}

// ── GET /api/v1/tools ───────────────────────────────────────────────

fn bench_get_tools(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let app = rt.block_on(test_app());

    c.bench_function("GET /api/v1/tools (2 tools)", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                let app = app.clone();
                async move {
                    let resp = app
                        .oneshot(Request::get("/api/v1/tools").body(Body::empty()).unwrap())
                        .await
                        .unwrap();
                    assert_eq!(resp.status(), 200);
                }
            });
    });
}

// ── POST /api/v1/crews ──────────────────────────────────────────────

fn bench_post_crews(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let app = rt.block_on(test_app());

    let body = serde_json::to_vec(&json!({
        "name": "bench-crew",
        "agents": [{
            "agent_key": "worker",
            "name": "Worker",
            "role": "worker",
            "goal": "execute tasks"
        }],
        "tasks": [{
            "description": "Run a benchmark task",
            "expected_output": "task result"
        }]
    }))
    .unwrap();

    c.bench_function("POST /api/v1/crews (1 task, placeholder)", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                let app = app.clone();
                let body = body.clone();
                async move {
                    let resp = app
                        .oneshot(
                            Request::post("/api/v1/crews")
                                .header("content-type", "application/json")
                                .body(Body::from(body))
                                .unwrap(),
                        )
                        .await
                        .unwrap();
                    assert_eq!(resp.status(), 200);
                }
            });
    });
}

// ── POST /mcp (initialize) ─────────────────────────────────────────

fn bench_post_mcp_initialize(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let app = rt.block_on(test_app());

    let body = serde_json::to_vec(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "clientInfo": {"name": "bench", "version": "0.1.0"}
        }
    }))
    .unwrap();

    c.bench_function("POST /mcp (initialize)", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| {
                let app = app.clone();
                let body = body.clone();
                async move {
                    let resp = app
                        .oneshot(
                            Request::post("/mcp")
                                .header("content-type", "application/json")
                                .body(Body::from(body))
                                .unwrap(),
                        )
                        .await
                        .unwrap();
                    assert_eq!(resp.status(), 200);
                }
            });
    });
}

criterion_group!(
    benches,
    bench_get_health,
    bench_get_ready,
    bench_get_metrics,
    bench_get_tools,
    bench_post_crews,
    bench_post_mcp_initialize,
);
criterion_main!(benches);
