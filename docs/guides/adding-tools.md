# Adding a Native Tool

Tools are Rust structs implementing the `NativeTool` trait. They run in-process with zero overhead.

## The Trait

```rust
pub trait NativeTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn schema(&self) -> ToolSchema;
    fn execute(&self, input: ToolInput) -> Pin<Box<dyn Future<Output = ToolOutput> + Send + '_>>;
}
```

## Example: HTTP Health Check Tool

```rust
use agnosai::tools::{NativeTool, ToolSchema, ParameterSchema, ToolInput, ToolOutput};
use std::pin::Pin;
use std::future::Future;

pub struct HealthCheckTool {
    client: reqwest::Client,
}

impl HealthCheckTool {
    pub fn new() -> Self {
        Self { client: reqwest::Client::new() }
    }
}

impl NativeTool for HealthCheckTool {
    fn name(&self) -> &str { "health_check" }

    fn description(&self) -> &str {
        "Check if a URL is reachable and return its HTTP status"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: vec![
                ParameterSchema {
                    name: "url".into(),
                    description: "URL to check".into(),
                    param_type: "string".into(),
                    required: true,
                },
            ],
        }
    }

    fn execute(&self, input: ToolInput) -> Pin<Box<dyn Future<Output = ToolOutput> + Send + '_>> {
        Box::pin(async move {
            let url = match input.get_str("url") {
                Some(u) => u,
                None => return ToolOutput::err("missing required parameter: url"),
            };

            match self.client.get(url).send().await {
                Ok(resp) => ToolOutput::ok(serde_json::json!({
                    "status": resp.status().as_u16(),
                    "ok": resp.status().is_success(),
                })),
                Err(e) => ToolOutput::err(format!("request failed: {e}")),
            }
        })
    }
}
```

## Registering the Tool

```rust
use agnosai::tools::ToolRegistry;
use std::sync::Arc;

let registry = ToolRegistry::new();
registry.register(Arc::new(HealthCheckTool::new()));

// Use it
let tool = registry.get("health_check").unwrap();
let input = ToolInput {
    parameters: [("url".into(), json!("https://example.com"))].into(),
};
let output = tool.execute(input).await;
```

## Adding to Built-ins

To include your tool in the default set:

1. Create `src/tools/builtin/your_tool.rs`
2. Export it in `src/tools/builtin/mod.rs`
3. Register it in `src/main.rs` (follow the pattern of EchoTool, JsonTransformTool)

## AGNOS Service Tools Pattern

For tools that talk to external HTTP services, follow the Synapse/Mneme/Delta pattern:

```rust
pub struct YourServiceTool {
    client: reqwest::Client,
    base_url: String,
}

impl YourServiceTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "http://localhost:PORT".to_string(),
        }
    }

    pub fn with_base_url(base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
        }
    }
}
```

This gives callers a zero-config default with the option to point at a different endpoint.
