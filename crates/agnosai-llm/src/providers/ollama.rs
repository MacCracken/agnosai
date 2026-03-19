//! Ollama provider via direct HTTP (reqwest).
//!
//! Targets a local Ollama instance at `http://localhost:11434` by default.

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::provider::{
    InferenceRequest, InferenceResponse, LlmProvider, ModelInfo, ProviderType, Result, TokenUsage,
};

const DEFAULT_BASE_URL: &str = "http://localhost:11434";

/// Ollama local-inference provider.
pub struct OllamaProvider {
    client: Client,
    base_url: String,
}

impl OllamaProvider {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    pub fn with_base_url(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new()
    }
}

// ── Ollama API types ────────────────────────────────────────────────

#[derive(Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

#[derive(Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
}

#[derive(Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    message: OllamaMessage,
    model: String,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
    #[serde(default)]
    eval_count: Option<u32>,
}

#[derive(Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModelEntry>,
}

#[derive(Deserialize)]
struct OllamaModelEntry {
    name: String,
}

// ── LlmProvider impl ───────────────────────────────────────────────

impl LlmProvider for OllamaProvider {
    async fn infer(&self, request: InferenceRequest) -> Result<InferenceResponse> {
        let url = format!("{}/api/chat", self.base_url);

        let options = if request.temperature.is_some() || request.max_tokens.is_some() {
            Some(OllamaOptions {
                temperature: request.temperature,
                num_predict: request.max_tokens,
            })
        } else {
            None
        };

        let body = OllamaChatRequest {
            model: request.model.clone(),
            messages: request
                .messages
                .iter()
                .map(|m| OllamaMessage {
                    role: m.role.clone(),
                    content: m.content.clone(),
                })
                .collect(),
            stream: false,
            options,
        };

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| agnosai_core::AgnosaiError::LlmProvider(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(agnosai_core::AgnosaiError::LlmProvider(format!(
                "Ollama HTTP {status}: {text}"
            )));
        }

        let ol: OllamaChatResponse = resp
            .json()
            .await
            .map_err(|e| agnosai_core::AgnosaiError::LlmProvider(e.to_string()))?;

        let prompt_tokens = ol.prompt_eval_count.unwrap_or(0);
        let completion_tokens = ol.eval_count.unwrap_or(0);

        Ok(InferenceResponse {
            content: ol.message.content,
            model: ol.model,
            usage: TokenUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            },
        })
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let url = format!("{}/api/tags", self.base_url);

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| agnosai_core::AgnosaiError::LlmProvider(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(agnosai_core::AgnosaiError::LlmProvider(format!(
                "Ollama HTTP {status}: {text}"
            )));
        }

        let tags: OllamaTagsResponse = resp
            .json()
            .await
            .map_err(|e| agnosai_core::AgnosaiError::LlmProvider(e.to_string()))?;

        Ok(tags
            .models
            .into_iter()
            .map(|m| ModelInfo {
                id: m.name.clone(),
                name: m.name,
                provider: ProviderType::Ollama,
            })
            .collect())
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Ollama
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construct_default() {
        let p = OllamaProvider::new();
        assert_eq!(p.base_url, "http://localhost:11434");
        assert_eq!(p.provider_type(), ProviderType::Ollama);
    }

    #[test]
    fn construct_with_base_url() {
        let p = OllamaProvider::with_base_url("http://gpu-server:11434".into());
        assert_eq!(p.base_url, "http://gpu-server:11434");
    }

    #[test]
    fn default_trait() {
        let p = OllamaProvider::default();
        assert_eq!(p.base_url, DEFAULT_BASE_URL);
    }
}
