//! MCP (Model Context Protocol) server — JSON-RPC 2.0 over HTTP POST.

use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::tools::ToolInput;

use crate::server::state::SharedState;

/// Inbound JSON-RPC 2.0 request for the MCP endpoint.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct JsonRpcRequest {
    /// JSON-RPC version (must be "2.0").
    pub jsonrpc: String,
    /// Request identifier for correlating responses.
    pub id: Value,
    /// Method name to invoke.
    pub method: String,
    /// Optional parameters for the method.
    #[serde(default)]
    pub params: Value,
}

/// Outbound JSON-RPC 2.0 response.
#[derive(Serialize)]
#[non_exhaustive]
pub struct JsonRpcResponse {
    /// JSON-RPC version (always "2.0").
    pub jsonrpc: String,
    /// Request identifier echoed back.
    pub id: Value,
    /// Result payload on success.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Error payload on failure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC error object.
#[derive(Serialize)]
pub struct JsonRpcError {
    /// Numeric error code.
    pub code: i32,
    /// Human-readable error message.
    pub message: String,
}

impl JsonRpcResponse {
    fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error(id: Value, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
            }),
        }
    }
}

/// POST /mcp — Handle an MCP JSON-RPC 2.0 request.
pub async fn mcp_handler(
    State(state): State<SharedState>,
    Json(req): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    tracing::debug!(method = %req.method, "MCP request");
    Json(match req.method.as_str() {
        "initialize" => handle_initialize(req.id),
        "tools/list" => handle_tools_list(req.id, &state),
        "tools/call" => handle_tools_call(req.id, &req.params, &state).await,
        _ => {
            tracing::warn!(method = %req.method, "MCP unknown method");
            JsonRpcResponse::error(req.id, -32601, "Method not found")
        }
    })
}

fn handle_initialize(id: Value) -> JsonRpcResponse {
    JsonRpcResponse::success(
        id,
        json!({
            "protocolVersion": "2024-11-05",
            "serverInfo": {
                "name": "agnosai",
                "version": "0.1.0"
            },
            "capabilities": {
                "tools": {}
            }
        }),
    )
}

fn handle_tools_list(id: Value, state: &SharedState) -> JsonRpcResponse {
    let schemas = state.tools.list();
    let tools: Vec<Value> = schemas
        .into_iter()
        .map(|schema| {
            let mut properties = serde_json::Map::new();
            let mut required = Vec::new();

            for param in &schema.parameters {
                properties.insert(
                    param.name.clone(),
                    json!({
                        "type": param.param_type,
                        "description": param.description,
                    }),
                );
                if param.required {
                    required.push(Value::String(param.name.clone()));
                }
            }

            json!({
                "name": schema.name,
                "description": schema.description,
                "inputSchema": {
                    "type": "object",
                    "properties": properties,
                    "required": required,
                }
            })
        })
        .collect();

    JsonRpcResponse::success(id, json!({ "tools": tools }))
}

async fn handle_tools_call(id: Value, params: &Value, state: &SharedState) -> JsonRpcResponse {
    let name = match params.get("name").and_then(|v| v.as_str()) {
        Some(n) => n,
        None => return JsonRpcResponse::error(id, -32602, "Missing tool name"),
    };

    let tool = match state.tools.get(name) {
        Some(t) => t,
        None => {
            return JsonRpcResponse::success(
                id,
                json!({
                    "content": [{"type": "text", "text": format!("Tool not found: {name}")}],
                    "isError": true
                }),
            );
        }
    };

    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or(Value::Object(Default::default()));

    let parameters = match arguments.as_object() {
        Some(map) => map.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        None => Default::default(),
    };

    let input = ToolInput { parameters };
    let start = std::time::Instant::now();
    let output = tool.execute(input).await;
    let elapsed = start.elapsed();

    tracing::info!(
        tool = name,
        success = output.success,
        duration_ms = elapsed.as_millis() as u64,
        "MCP tool call"
    );

    if output.success {
        let text = match output.result {
            Value::String(s) => s,
            other => other.to_string(),
        };
        JsonRpcResponse::success(
            id,
            json!({
                "content": [{"type": "text", "text": text}],
                "isError": false
            }),
        )
    } else {
        let text = output.error.unwrap_or_else(|| "Unknown error".to_string());
        JsonRpcResponse::success(
            id,
            json!({
                "content": [{"type": "text", "text": text}],
                "isError": true
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::Orchestrator;
    use crate::server::state::{AppState, SharedState};
    use crate::tools::ToolRegistry;
    use crate::tools::builtin::echo::EchoTool;
    use axum::Router;
    use axum::http::{Request, StatusCode};
    use std::sync::Arc;
    use tower::ServiceExt;

    async fn test_app() -> Router {
        let orchestrator = Orchestrator::new(Default::default()).await.unwrap();
        let tools = Arc::new(ToolRegistry::new());
        tools.register(Arc::new(EchoTool));
        let state: SharedState = Arc::new(AppState {
            orchestrator,
            tools,
            auth: Default::default(),
            events: crate::server::sse::EventBus::new(),
            http_client: reqwest::Client::new(),
        });
        crate::server::router(state)
    }

    async fn rpc(app: Router, body: Value) -> (StatusCode, Value) {
        let response = app
            .oneshot(
                Request::post("/mcp")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        (status, json)
    }

    #[tokio::test]
    async fn initialize_returns_server_info_and_capabilities() {
        let app = test_app().await;
        let (status, json) = rpc(
            app,
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "clientInfo": {"name": "test", "version": "0.1.0"}
                }
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["id"], 1);
        assert_eq!(json["result"]["protocolVersion"], "2024-11-05");
        assert_eq!(json["result"]["serverInfo"]["name"], "agnosai");
        assert_eq!(json["result"]["serverInfo"]["version"], "0.1.0");
        assert!(json["result"]["capabilities"]["tools"].is_object());
    }

    #[tokio::test]
    async fn tools_list_returns_registered_tools_in_mcp_format() {
        let app = test_app().await;
        let (status, json) = rpc(
            app,
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        let tools = json["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "echo");
        assert!(!tools[0]["description"].as_str().unwrap().is_empty());
        // Verify inputSchema structure
        let schema = &tools[0]["inputSchema"];
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["message"].is_object());
        assert_eq!(schema["properties"]["message"]["type"], "string");
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("message")));
    }

    #[tokio::test]
    async fn tools_call_executes_echo_tool() {
        let app = test_app().await;
        let (status, json) = rpc(
            app,
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {"name": "echo", "arguments": {"message": "hello"}}
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["result"]["isError"], false);
        let content = json["result"]["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0]["text"], "hello");
    }

    #[tokio::test]
    async fn tools_call_unknown_tool_returns_is_error() {
        let app = test_app().await;
        let (status, json) = rpc(
            app,
            json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "tools/call",
                "params": {"name": "nonexistent", "arguments": {}}
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["result"]["isError"], true);
        let content = json["result"]["content"].as_array().unwrap();
        assert!(content[0]["text"].as_str().unwrap().contains("not found"));
    }

    #[tokio::test]
    async fn unknown_method_returns_32601() {
        let app = test_app().await;
        let (status, json) = rpc(
            app,
            json!({"jsonrpc": "2.0", "id": 5, "method": "bogus/method"}),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["error"]["code"], -32601);
        assert_eq!(json["error"]["message"], "Method not found");
        assert!(json.get("result").is_none());
    }
}
