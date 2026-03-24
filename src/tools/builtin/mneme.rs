//! Mneme Knowledge Base tools.
//!
//! Mneme provides a personal knowledge base with full-text search, backlinks,
//! and tagging. Default base URL: `http://localhost:8400`.

use crate::tools::native::{NativeTool, ParameterSchema, ToolInput, ToolOutput, ToolSchema};
use reqwest::Client;
use serde_json::{Value, json};
use std::future::Future;
use std::pin::Pin;
use std::sync::OnceLock;

const DEFAULT_BASE_URL: &str = "http://localhost:8400";

/// Shared HTTP client for all mneme tools.
fn shared_client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(Client::new)
}

// ---------------------------------------------------------------------------
// mneme_search
// ---------------------------------------------------------------------------

/// Search the Mneme knowledge base.
pub struct MnemeSearch {
    client: Client,
    base_url: String,
}

impl Default for MnemeSearch {
    fn default() -> Self {
        Self::new()
    }
}

impl MnemeSearch {
    /// Create a new instance with the default base URL.
    pub fn new() -> Self {
        Self::with_base_url(DEFAULT_BASE_URL.to_string())
    }

    /// Create a new instance targeting the given base URL.
    pub fn with_base_url(base_url: String) -> Self {
        Self {
            client: shared_client().clone(),
            base_url,
        }
    }
}

impl NativeTool for MnemeSearch {
    fn name(&self) -> &str {
        "mneme_search"
    }

    fn description(&self) -> &str {
        "Search the Mneme knowledge base for notes matching a query"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_owned(),
            description: self.description().to_owned(),
            parameters: vec![
                ParameterSchema {
                    name: "query".to_owned(),
                    description: "Search query text".to_owned(),
                    param_type: "string".to_owned(),
                    required: true,
                },
                ParameterSchema {
                    name: "limit".to_owned(),
                    description: "Max results to return (default 10)".to_owned(),
                    param_type: "number".to_owned(),
                    required: false,
                },
            ],
        }
    }

    fn execute(&self, input: ToolInput) -> Pin<Box<dyn Future<Output = ToolOutput> + Send + '_>> {
        Box::pin(async move {
            let query = match input.get_str("query") {
                Some(q) => q.to_string(),
                None => return ToolOutput::err("missing required parameter: query"),
            };
            let limit = input.get_u64("limit").unwrap_or(10);

            let url = format!("{}/api/search", self.base_url);
            match self
                .client
                .get(&url)
                .query(&[("q", query.as_str()), ("limit", &limit.to_string())])
                .send()
                .await
            {
                Ok(resp) => match resp.json::<Value>().await {
                    Ok(data) => ToolOutput::ok(data),
                    Err(e) => ToolOutput::err(format!("failed to parse response: {e}")),
                },
                Err(e) => ToolOutput::err(format!("mneme request failed: {e}")),
            }
        })
    }
}

// ---------------------------------------------------------------------------
// mneme_get_note
// ---------------------------------------------------------------------------

/// Retrieve a single note by ID from Mneme.
pub struct MnemeGetNote {
    client: Client,
    base_url: String,
}

impl Default for MnemeGetNote {
    fn default() -> Self {
        Self::new()
    }
}

impl MnemeGetNote {
    /// Create a new instance with the default base URL.
    pub fn new() -> Self {
        Self::with_base_url(DEFAULT_BASE_URL.to_string())
    }

    /// Create a new instance targeting the given base URL.
    pub fn with_base_url(base_url: String) -> Self {
        Self {
            client: shared_client().clone(),
            base_url,
        }
    }
}

impl NativeTool for MnemeGetNote {
    fn name(&self) -> &str {
        "mneme_get_note"
    }

    fn description(&self) -> &str {
        "Get a note by ID from the Mneme knowledge base"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_owned(),
            description: self.description().to_owned(),
            parameters: vec![ParameterSchema {
                name: "note_id".to_owned(),
                description: "Note identifier".to_owned(),
                param_type: "string".to_owned(),
                required: true,
            }],
        }
    }

    fn execute(&self, input: ToolInput) -> Pin<Box<dyn Future<Output = ToolOutput> + Send + '_>> {
        Box::pin(async move {
            let note_id = match input.get_str("note_id") {
                Some(id) => id.to_string(),
                None => return ToolOutput::err("missing required parameter: note_id"),
            };

            if note_id.contains('/') || note_id.contains("..") {
                return ToolOutput::err("note_id contains invalid characters");
            }
            let url = format!("{}/api/notes/{}", self.base_url, note_id);
            match self.client.get(&url).send().await {
                Ok(resp) => match resp.json::<Value>().await {
                    Ok(data) => ToolOutput::ok(data),
                    Err(e) => ToolOutput::err(format!("failed to parse response: {e}")),
                },
                Err(e) => ToolOutput::err(format!("mneme request failed: {e}")),
            }
        })
    }
}

// ---------------------------------------------------------------------------
// mneme_create_note
// ---------------------------------------------------------------------------

/// Create a new note in Mneme (useful for agents storing findings).
pub struct MnemeCreateNote {
    client: Client,
    base_url: String,
}

impl Default for MnemeCreateNote {
    fn default() -> Self {
        Self::new()
    }
}

impl MnemeCreateNote {
    /// Create a new instance with the default base URL.
    pub fn new() -> Self {
        Self::with_base_url(DEFAULT_BASE_URL.to_string())
    }

    /// Create a new instance targeting the given base URL.
    pub fn with_base_url(base_url: String) -> Self {
        Self {
            client: shared_client().clone(),
            base_url,
        }
    }
}

impl NativeTool for MnemeCreateNote {
    fn name(&self) -> &str {
        "mneme_create_note"
    }

    fn description(&self) -> &str {
        "Create a new note in the Mneme knowledge base"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_owned(),
            description: self.description().to_owned(),
            parameters: vec![
                ParameterSchema {
                    name: "title".to_owned(),
                    description: "Note title".to_owned(),
                    param_type: "string".to_owned(),
                    required: true,
                },
                ParameterSchema {
                    name: "content".to_owned(),
                    description: "Note body content (Markdown)".to_owned(),
                    param_type: "string".to_owned(),
                    required: true,
                },
                ParameterSchema {
                    name: "tags".to_owned(),
                    description: "Optional tags for categorisation".to_owned(),
                    param_type: "array".to_owned(),
                    required: false,
                },
            ],
        }
    }

    fn execute(&self, input: ToolInput) -> Pin<Box<dyn Future<Output = ToolOutput> + Send + '_>> {
        Box::pin(async move {
            let title = match input.get_str("title") {
                Some(t) => t.to_string(),
                None => return ToolOutput::err("missing required parameter: title"),
            };
            let content = match input.get_str("content") {
                Some(c) => c.to_string(),
                None => return ToolOutput::err("missing required parameter: content"),
            };
            let tags = input
                .parameters
                .get("tags")
                .cloned()
                .unwrap_or_else(|| json!([]));

            let body = json!({
                "title": title,
                "content": content,
                "tags": tags,
            });

            let url = format!("{}/api/notes", self.base_url);
            match self.client.post(&url).json(&body).send().await {
                Ok(resp) => match resp.json::<Value>().await {
                    Ok(data) => ToolOutput::ok(data),
                    Err(e) => ToolOutput::err(format!("failed to parse response: {e}")),
                },
                Err(e) => ToolOutput::err(format!("mneme request failed: {e}")),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // ── MnemeSearch ─────────────────────────────────────────────────────

    #[test]
    fn mneme_search_name() {
        assert_eq!(MnemeSearch::new().name(), "mneme_search");
    }

    #[test]
    fn mneme_search_description_non_empty() {
        assert!(!MnemeSearch::new().description().is_empty());
    }

    #[test]
    fn mneme_search_schema_parameters() {
        let schema = MnemeSearch::new().schema();
        assert_eq!(schema.name, "mneme_search");
        assert_eq!(schema.parameters.len(), 2);

        let query = schema
            .parameters
            .iter()
            .find(|p| p.name == "query")
            .unwrap();
        assert_eq!(query.param_type, "string");
        assert!(query.required);

        let limit = schema
            .parameters
            .iter()
            .find(|p| p.name == "limit")
            .unwrap();
        assert_eq!(limit.param_type, "number");
        assert!(!limit.required);
    }

    #[tokio::test]
    async fn mneme_search_missing_query() {
        let tool = MnemeSearch::new();
        let output = tool
            .execute(ToolInput {
                parameters: HashMap::new(),
            })
            .await;
        assert!(!output.success);
        assert!(output.error.unwrap().contains("query"));
    }

    // ── MnemeGetNote ────────────────────────────────────────────────────

    #[test]
    fn mneme_get_note_name() {
        assert_eq!(MnemeGetNote::new().name(), "mneme_get_note");
    }

    #[test]
    fn mneme_get_note_description_non_empty() {
        assert!(!MnemeGetNote::new().description().is_empty());
    }

    #[test]
    fn mneme_get_note_schema_parameters() {
        let schema = MnemeGetNote::new().schema();
        assert_eq!(schema.name, "mneme_get_note");
        assert_eq!(schema.parameters.len(), 1);

        let note_id = &schema.parameters[0];
        assert_eq!(note_id.name, "note_id");
        assert_eq!(note_id.param_type, "string");
        assert!(note_id.required);
    }

    #[tokio::test]
    async fn mneme_get_note_missing_note_id() {
        let tool = MnemeGetNote::new();
        let output = tool
            .execute(ToolInput {
                parameters: HashMap::new(),
            })
            .await;
        assert!(!output.success);
        assert!(output.error.unwrap().contains("note_id"));
    }

    // ── MnemeCreateNote ─────────────────────────────────────────────────

    #[test]
    fn mneme_create_note_name() {
        assert_eq!(MnemeCreateNote::new().name(), "mneme_create_note");
    }

    #[test]
    fn mneme_create_note_description_non_empty() {
        assert!(!MnemeCreateNote::new().description().is_empty());
    }

    #[test]
    fn mneme_create_note_schema_parameters() {
        let schema = MnemeCreateNote::new().schema();
        assert_eq!(schema.name, "mneme_create_note");
        assert_eq!(schema.parameters.len(), 3);

        let title = schema
            .parameters
            .iter()
            .find(|p| p.name == "title")
            .unwrap();
        assert_eq!(title.param_type, "string");
        assert!(title.required);

        let content = schema
            .parameters
            .iter()
            .find(|p| p.name == "content")
            .unwrap();
        assert_eq!(content.param_type, "string");
        assert!(content.required);

        let tags = schema.parameters.iter().find(|p| p.name == "tags").unwrap();
        assert_eq!(tags.param_type, "array");
        assert!(!tags.required);
    }

    #[tokio::test]
    async fn mneme_create_note_missing_title() {
        let tool = MnemeCreateNote::new();
        let mut params = HashMap::new();
        params.insert("content".to_owned(), json!("body text"));
        let output = tool.execute(ToolInput { parameters: params }).await;
        assert!(!output.success);
        assert!(output.error.unwrap().contains("title"));
    }

    #[tokio::test]
    async fn mneme_create_note_missing_content() {
        let tool = MnemeCreateNote::new();
        let mut params = HashMap::new();
        params.insert("title".to_owned(), json!("My Note"));
        let output = tool.execute(ToolInput { parameters: params }).await;
        assert!(!output.success);
        assert!(output.error.unwrap().contains("content"));
    }

    #[tokio::test]
    async fn mneme_create_note_missing_all_required() {
        let tool = MnemeCreateNote::new();
        let output = tool
            .execute(ToolInput {
                parameters: HashMap::new(),
            })
            .await;
        assert!(!output.success);
        assert!(output.error.is_some());
    }
}
