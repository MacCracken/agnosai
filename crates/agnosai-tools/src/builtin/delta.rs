//! Delta Code Platform tools.
//!
//! Delta provides Git hosting and CI/CD pipelines. Default base URL:
//! `http://localhost:8070`.

use crate::native::{NativeTool, ParameterSchema, ToolInput, ToolOutput, ToolSchema};
use reqwest::Client;
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;

const DEFAULT_BASE_URL: &str = "http://localhost:8070";

// ---------------------------------------------------------------------------
// delta_list_repos
// ---------------------------------------------------------------------------

/// List repositories on the Delta code platform.
pub struct DeltaListRepos {
    client: Client,
    base_url: String,
}

impl DeltaListRepos {
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

impl NativeTool for DeltaListRepos {
    fn name(&self) -> &str {
        "delta_list_repos"
    }

    fn description(&self) -> &str {
        "List repositories on the Delta code platform"
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
            let url = format!("{}/api/v1/repos", self.base_url);
            match self.client.get(&url).send().await {
                Ok(resp) => match resp.json::<Value>().await {
                    Ok(data) => ToolOutput::ok(data),
                    Err(e) => ToolOutput::err(format!("failed to parse response: {e}")),
                },
                Err(e) => ToolOutput::err(format!("delta request failed: {e}")),
            }
        })
    }
}

// ---------------------------------------------------------------------------
// delta_trigger_pipeline
// ---------------------------------------------------------------------------

/// Trigger a CI/CD pipeline on Delta.
pub struct DeltaTriggerPipeline {
    client: Client,
    base_url: String,
}

impl DeltaTriggerPipeline {
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

impl NativeTool for DeltaTriggerPipeline {
    fn name(&self) -> &str {
        "delta_trigger_pipeline"
    }

    fn description(&self) -> &str {
        "Trigger a CI/CD pipeline for a repository on the Delta code platform"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_owned(),
            description: self.description().to_owned(),
            parameters: vec![
                ParameterSchema {
                    name: "owner".to_owned(),
                    description: "Repository owner".to_owned(),
                    param_type: "string".to_owned(),
                    required: true,
                },
                ParameterSchema {
                    name: "repo".to_owned(),
                    description: "Repository name".to_owned(),
                    param_type: "string".to_owned(),
                    required: true,
                },
                ParameterSchema {
                    name: "branch".to_owned(),
                    description: "Branch to build (defaults to main)".to_owned(),
                    param_type: "string".to_owned(),
                    required: false,
                },
            ],
        }
    }

    fn execute(&self, input: ToolInput) -> Pin<Box<dyn Future<Output = ToolOutput> + Send + '_>> {
        Box::pin(async move {
            let owner = match input.get_str("owner") {
                Some(o) => o.to_string(),
                None => return ToolOutput::err("missing required parameter: owner"),
            };
            let repo = match input.get_str("repo") {
                Some(r) => r.to_string(),
                None => return ToolOutput::err("missing required parameter: repo"),
            };

            let mut body = json!({});
            if let Some(branch) = input.get_str("branch") {
                body["branch"] = json!(branch);
            }

            let url = format!("{}/api/v1/{}/{}/pipelines", self.base_url, owner, repo);
            match self.client.post(&url).json(&body).send().await {
                Ok(resp) => match resp.json::<Value>().await {
                    Ok(data) => ToolOutput::ok(data),
                    Err(e) => ToolOutput::err(format!("failed to parse response: {e}")),
                },
                Err(e) => ToolOutput::err(format!("delta request failed: {e}")),
            }
        })
    }
}

// ---------------------------------------------------------------------------
// delta_get_pipeline
// ---------------------------------------------------------------------------

/// Get pipeline status from Delta.
pub struct DeltaGetPipeline {
    client: Client,
    base_url: String,
}

impl DeltaGetPipeline {
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

impl NativeTool for DeltaGetPipeline {
    fn name(&self) -> &str {
        "delta_get_pipeline"
    }

    fn description(&self) -> &str {
        "Get the status of a CI/CD pipeline run on the Delta code platform"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_owned(),
            description: self.description().to_owned(),
            parameters: vec![
                ParameterSchema {
                    name: "owner".to_owned(),
                    description: "Repository owner".to_owned(),
                    param_type: "string".to_owned(),
                    required: true,
                },
                ParameterSchema {
                    name: "repo".to_owned(),
                    description: "Repository name".to_owned(),
                    param_type: "string".to_owned(),
                    required: true,
                },
                ParameterSchema {
                    name: "pipeline_id".to_owned(),
                    description: "Pipeline run identifier".to_owned(),
                    param_type: "string".to_owned(),
                    required: true,
                },
            ],
        }
    }

    fn execute(&self, input: ToolInput) -> Pin<Box<dyn Future<Output = ToolOutput> + Send + '_>> {
        Box::pin(async move {
            let owner = match input.get_str("owner") {
                Some(o) => o.to_string(),
                None => return ToolOutput::err("missing required parameter: owner"),
            };
            let repo = match input.get_str("repo") {
                Some(r) => r.to_string(),
                None => return ToolOutput::err("missing required parameter: repo"),
            };
            let pipeline_id = match input.get_str("pipeline_id") {
                Some(p) => p.to_string(),
                None => return ToolOutput::err("missing required parameter: pipeline_id"),
            };

            let url = format!(
                "{}/api/v1/{}/{}/pipelines/{}",
                self.base_url, owner, repo, pipeline_id
            );
            match self.client.get(&url).send().await {
                Ok(resp) => match resp.json::<Value>().await {
                    Ok(data) => ToolOutput::ok(data),
                    Err(e) => ToolOutput::err(format!("failed to parse response: {e}")),
                },
                Err(e) => ToolOutput::err(format!("delta request failed: {e}")),
            }
        })
    }
}
