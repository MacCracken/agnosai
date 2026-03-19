//! Mneme Knowledge Base tools.
//!
//! Mneme provides a personal knowledge base with full-text search, backlinks,
//! and tagging. Default base URL: `http://localhost:8400`.

use crate::native::{NativeTool, ParameterSchema, ToolInput, ToolOutput, ToolSchema};
use reqwest::Client;
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;

const DEFAULT_BASE_URL: &str = "http://localhost:8400";

// ---------------------------------------------------------------------------
// mneme_search
// ---------------------------------------------------------------------------

/// Search the Mneme knowledge base.
pub struct MnemeSearch {
    client: Client,
    base_url: String,
}

impl MnemeSearch {
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

impl MnemeGetNote {
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

impl MnemeCreateNote {
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
