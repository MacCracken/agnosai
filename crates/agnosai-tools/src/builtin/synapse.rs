//! Synapse LLM Controller tools.
//!
//! Synapse exposes an OpenAI-compatible API for local/remote model inference.
//! Default base URL: `http://localhost:8420`.

use crate::native::{NativeTool, ParameterSchema, ToolInput, ToolOutput, ToolSchema};
use reqwest::Client;
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;

const DEFAULT_BASE_URL: &str = "http://localhost:8420";

// ---------------------------------------------------------------------------
// synapse_infer
// ---------------------------------------------------------------------------

/// Run inference through Synapse's OpenAI-compatible chat completions endpoint.
pub struct SynapseInfer {
    client: Client,
    base_url: String,
}

impl SynapseInfer {
    pub fn new() -> Self {
        Self::with_base_url(DEFAULT_BASE_URL.to_string())
    }

    pub fn with_base_url(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }
}

impl NativeTool for SynapseInfer {
    fn name(&self) -> &str {
        "synapse_infer"
    }

    fn description(&self) -> &str {
        "Run inference through the Synapse LLM controller (OpenAI-compatible chat completions)"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_owned(),
            description: self.description().to_owned(),
            parameters: vec![
                ParameterSchema {
                    name: "model".to_owned(),
                    description: "Model identifier".to_owned(),
                    param_type: "string".to_owned(),
                    required: true,
                },
                ParameterSchema {
                    name: "prompt".to_owned(),
                    description: "User prompt text".to_owned(),
                    param_type: "string".to_owned(),
                    required: true,
                },
                ParameterSchema {
                    name: "max_tokens".to_owned(),
                    description: "Maximum tokens to generate".to_owned(),
                    param_type: "number".to_owned(),
                    required: false,
                },
                ParameterSchema {
                    name: "temperature".to_owned(),
                    description: "Sampling temperature (0.0-2.0)".to_owned(),
                    param_type: "number".to_owned(),
                    required: false,
                },
            ],
        }
    }

    fn execute(&self, input: ToolInput) -> Pin<Box<dyn Future<Output = ToolOutput> + Send + '_>> {
        Box::pin(async move {
            let model = match input.get_str("model") {
                Some(m) => m.to_string(),
                None => return ToolOutput::err("missing required parameter: model"),
            };
            let prompt = match input.get_str("prompt") {
                Some(p) => p.to_string(),
                None => return ToolOutput::err("missing required parameter: prompt"),
            };

            let mut body = json!({
                "model": model,
                "messages": [{ "role": "user", "content": prompt }]
            });

            if let Some(max_tokens) = input.get_u64("max_tokens") {
                body["max_tokens"] = json!(max_tokens);
            }
            if let Some(temperature) = input.get_f64("temperature") {
                body["temperature"] = json!(temperature);
            }

            let url = format!("{}/v1/chat/completions", self.base_url);
            match self.client.post(&url).json(&body).send().await {
                Ok(resp) => match resp.json::<Value>().await {
                    Ok(data) => {
                        // Extract completion text from the OpenAI-compatible response.
                        let text = data["choices"]
                            .as_array()
                            .and_then(|c| c.first())
                            .and_then(|c| c["message"]["content"].as_str())
                            .unwrap_or("");
                        ToolOutput::ok(json!({ "completion": text, "raw": data }))
                    }
                    Err(e) => ToolOutput::err(format!("failed to parse response: {e}")),
                },
                Err(e) => ToolOutput::err(format!("synapse request failed: {e}")),
            }
        })
    }
}

// ---------------------------------------------------------------------------
// synapse_list_models
// ---------------------------------------------------------------------------

/// List models available through Synapse.
pub struct SynapseListModels {
    client: Client,
    base_url: String,
}

impl SynapseListModels {
    pub fn new() -> Self {
        Self::with_base_url(DEFAULT_BASE_URL.to_string())
    }

    pub fn with_base_url(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }
}

impl NativeTool for SynapseListModels {
    fn name(&self) -> &str {
        "synapse_list_models"
    }

    fn description(&self) -> &str {
        "List models available through the Synapse LLM controller"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_owned(),
            description: self.description().to_owned(),
            parameters: vec![],
        }
    }

    fn execute(
        &self,
        _input: ToolInput,
    ) -> Pin<Box<dyn Future<Output = ToolOutput> + Send + '_>> {
        Box::pin(async move {
            let url = format!("{}/v1/models", self.base_url);
            match self.client.get(&url).send().await {
                Ok(resp) => match resp.json::<Value>().await {
                    Ok(data) => ToolOutput::ok(data),
                    Err(e) => ToolOutput::err(format!("failed to parse response: {e}")),
                },
                Err(e) => ToolOutput::err(format!("synapse request failed: {e}")),
            }
        })
    }
}

// ---------------------------------------------------------------------------
// synapse_status
// ---------------------------------------------------------------------------

/// Get Synapse system status (loaded models, backends, hardware).
pub struct SynapseStatus {
    client: Client,
    base_url: String,
}

impl SynapseStatus {
    pub fn new() -> Self {
        Self::with_base_url(DEFAULT_BASE_URL.to_string())
    }

    pub fn with_base_url(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }
}

impl NativeTool for SynapseStatus {
    fn name(&self) -> &str {
        "synapse_status"
    }

    fn description(&self) -> &str {
        "Get Synapse system status: loaded models, backends, and hardware info"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_owned(),
            description: self.description().to_owned(),
            parameters: vec![],
        }
    }

    fn execute(
        &self,
        _input: ToolInput,
    ) -> Pin<Box<dyn Future<Output = ToolOutput> + Send + '_>> {
        Box::pin(async move {
            let url = format!("{}/system/status", self.base_url);
            match self.client.get(&url).send().await {
                Ok(resp) => match resp.json::<Value>().await {
                    Ok(data) => ToolOutput::ok(data),
                    Err(e) => ToolOutput::err(format!("failed to parse response: {e}")),
                },
                Err(e) => ToolOutput::err(format!("synapse request failed: {e}")),
            }
        })
    }
}
