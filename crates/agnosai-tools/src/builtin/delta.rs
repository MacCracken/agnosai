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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // ── DeltaListRepos ──────────────────────────────────────────────────

    #[test]
    fn delta_list_repos_name() {
        assert_eq!(DeltaListRepos::new().name(), "delta_list_repos");
    }

    #[test]
    fn delta_list_repos_description_non_empty() {
        assert!(!DeltaListRepos::new().description().is_empty());
    }

    #[test]
    fn delta_list_repos_schema_no_params() {
        let schema = DeltaListRepos::new().schema();
        assert_eq!(schema.name, "delta_list_repos");
        assert!(schema.parameters.is_empty());
    }

    // ── DeltaTriggerPipeline ────────────────────────────────────────────

    #[test]
    fn delta_trigger_pipeline_name() {
        assert_eq!(DeltaTriggerPipeline::new().name(), "delta_trigger_pipeline");
    }

    #[test]
    fn delta_trigger_pipeline_description_non_empty() {
        assert!(!DeltaTriggerPipeline::new().description().is_empty());
    }

    #[test]
    fn delta_trigger_pipeline_schema_parameters() {
        let schema = DeltaTriggerPipeline::new().schema();
        assert_eq!(schema.name, "delta_trigger_pipeline");
        assert_eq!(schema.parameters.len(), 3);

        let owner = schema.parameters.iter().find(|p| p.name == "owner").unwrap();
        assert_eq!(owner.param_type, "string");
        assert!(owner.required);

        let repo = schema.parameters.iter().find(|p| p.name == "repo").unwrap();
        assert_eq!(repo.param_type, "string");
        assert!(repo.required);

        let branch = schema
            .parameters
            .iter()
            .find(|p| p.name == "branch")
            .unwrap();
        assert_eq!(branch.param_type, "string");
        assert!(!branch.required);
    }

    #[tokio::test]
    async fn delta_trigger_pipeline_missing_owner() {
        let tool = DeltaTriggerPipeline::new();
        let mut params = HashMap::new();
        params.insert("repo".to_owned(), json!("my-repo"));
        let output = tool.execute(ToolInput { parameters: params }).await;
        assert!(!output.success);
        assert!(output.error.unwrap().contains("owner"));
    }

    #[tokio::test]
    async fn delta_trigger_pipeline_missing_repo() {
        let tool = DeltaTriggerPipeline::new();
        let mut params = HashMap::new();
        params.insert("owner".to_owned(), json!("my-org"));
        let output = tool.execute(ToolInput { parameters: params }).await;
        assert!(!output.success);
        assert!(output.error.unwrap().contains("repo"));
    }

    #[tokio::test]
    async fn delta_trigger_pipeline_missing_all_required() {
        let tool = DeltaTriggerPipeline::new();
        let output = tool
            .execute(ToolInput {
                parameters: HashMap::new(),
            })
            .await;
        assert!(!output.success);
        assert!(output.error.is_some());
    }

    // ── DeltaGetPipeline ────────────────────────────────────────────────

    #[test]
    fn delta_get_pipeline_name() {
        assert_eq!(DeltaGetPipeline::new().name(), "delta_get_pipeline");
    }

    #[test]
    fn delta_get_pipeline_description_non_empty() {
        assert!(!DeltaGetPipeline::new().description().is_empty());
    }

    #[test]
    fn delta_get_pipeline_schema_parameters() {
        let schema = DeltaGetPipeline::new().schema();
        assert_eq!(schema.name, "delta_get_pipeline");
        assert_eq!(schema.parameters.len(), 3);

        let owner = schema.parameters.iter().find(|p| p.name == "owner").unwrap();
        assert_eq!(owner.param_type, "string");
        assert!(owner.required);

        let repo = schema.parameters.iter().find(|p| p.name == "repo").unwrap();
        assert_eq!(repo.param_type, "string");
        assert!(repo.required);

        let pipeline_id = schema
            .parameters
            .iter()
            .find(|p| p.name == "pipeline_id")
            .unwrap();
        assert_eq!(pipeline_id.param_type, "string");
        assert!(pipeline_id.required);
    }

    #[tokio::test]
    async fn delta_get_pipeline_missing_owner() {
        let tool = DeltaGetPipeline::new();
        let mut params = HashMap::new();
        params.insert("repo".to_owned(), json!("my-repo"));
        params.insert("pipeline_id".to_owned(), json!("123"));
        let output = tool.execute(ToolInput { parameters: params }).await;
        assert!(!output.success);
        assert!(output.error.unwrap().contains("owner"));
    }

    #[tokio::test]
    async fn delta_get_pipeline_missing_repo() {
        let tool = DeltaGetPipeline::new();
        let mut params = HashMap::new();
        params.insert("owner".to_owned(), json!("my-org"));
        params.insert("pipeline_id".to_owned(), json!("123"));
        let output = tool.execute(ToolInput { parameters: params }).await;
        assert!(!output.success);
        assert!(output.error.unwrap().contains("repo"));
    }

    #[tokio::test]
    async fn delta_get_pipeline_missing_pipeline_id() {
        let tool = DeltaGetPipeline::new();
        let mut params = HashMap::new();
        params.insert("owner".to_owned(), json!("my-org"));
        params.insert("repo".to_owned(), json!("my-repo"));
        let output = tool.execute(ToolInput { parameters: params }).await;
        assert!(!output.success);
        assert!(output.error.unwrap().contains("pipeline_id"));
    }

    #[tokio::test]
    async fn delta_get_pipeline_missing_all_required() {
        let tool = DeltaGetPipeline::new();
        let output = tool
            .execute(ToolInput {
                parameters: HashMap::new(),
            })
            .await;
        assert!(!output.success);
        assert!(output.error.is_some());
    }
}
