//! OpenAI-compatible provider via direct HTTP (reqwest).
//!
//! Works with OpenAI, Azure OpenAI, and any API-compatible endpoint
//! (e.g. vLLM, LiteLLM proxy) by setting a custom `base_url`.

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::provider::{
    InferenceRequest, InferenceResponse, LlmProvider, ModelInfo, ProviderType, Result, TokenUsage,
};

/// OpenAI-compatible LLM provider.
pub struct OpenAiProvider {
    client: Client,
    base_url: String,
    api_key: String,
    default_model: String,
}

impl OpenAiProvider {
    /// Create a provider targeting `https://api.openai.com` with `gpt-4o`.
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            base_url: "https://api.openai.com".to_string(),
            api_key,
            default_model: "gpt-4o".to_string(),
        }
    }

    /// Create a provider with a custom base URL (for compatible APIs).
    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            api_key,
            default_model: "gpt-4o".to_string(),
        }
    }

    /// Create a provider with a specific default model.
    pub fn with_model(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            base_url: "https://api.openai.com".to_string(),
            api_key,
            default_model: model,
        }
    }

    fn resolve_model(&self, model: &str) -> String {
        if model.is_empty() {
            self.default_model.clone()
        } else {
            model.to_string()
        }
    }
}

// ── OpenAI API types ────────────────────────────────────────────────

#[derive(Serialize)]
struct OaiChatRequest {
    model: String,
    messages: Vec<OaiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    stream: bool,
}

#[derive(Serialize, Deserialize)]
struct OaiMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OaiChatResponse {
    choices: Vec<OaiChoice>,
    model: String,
    #[serde(default)]
    usage: Option<OaiUsage>,
}

#[derive(Deserialize)]
struct OaiChoice {
    message: OaiMessage,
}

#[derive(Deserialize)]
struct OaiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Deserialize)]
struct OaiModelsResponse {
    data: Vec<OaiModel>,
}

#[derive(Deserialize)]
struct OaiModel {
    id: String,
}

#[derive(Deserialize)]
struct OaiErrorResponse {
    error: OaiErrorDetail,
}

#[derive(Deserialize)]
struct OaiErrorDetail {
    message: String,
}

// ── LlmProvider impl ───────────────────────────────────────────────

impl LlmProvider for OpenAiProvider {
    async fn infer(&self, request: InferenceRequest) -> Result<InferenceResponse> {
        let model = self.resolve_model(&request.model);
        let url = format!("{}/v1/chat/completions", self.base_url);

        let body = OaiChatRequest {
            model: model.clone(),
            messages: request
                .messages
                .iter()
                .map(|m| OaiMessage {
                    role: m.role.clone(),
                    content: m.content.clone(),
                })
                .collect(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: false, // streaming handled separately
        };

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| agnosai_core::AgnosaiError::LlmProvider(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".to_string());
            let detail = serde_json::from_str::<OaiErrorResponse>(&text)
                .map(|e| e.error.message)
                .unwrap_or(text);
            return Err(agnosai_core::AgnosaiError::LlmProvider(format!(
                "OpenAI HTTP {status}: {detail}"
            )));
        }

        let oai: OaiChatResponse = resp
            .json()
            .await
            .map_err(|e| agnosai_core::AgnosaiError::LlmProvider(e.to_string()))?;

        let content = oai
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        let usage = oai.usage.map_or_else(TokenUsage::default, |u| TokenUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });

        Ok(InferenceResponse {
            content,
            model: oai.model,
            usage,
        })
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let url = format!("{}/v1/models", self.base_url);

        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|e| agnosai_core::AgnosaiError::LlmProvider(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(agnosai_core::AgnosaiError::LlmProvider(format!(
                "OpenAI HTTP {status}: {text}"
            )));
        }

        let oai: OaiModelsResponse = resp
            .json()
            .await
            .map_err(|e| agnosai_core::AgnosaiError::LlmProvider(e.to_string()))?;

        Ok(oai
            .data
            .into_iter()
            .map(|m| ModelInfo {
                id: m.id.clone(),
                name: m.id,
                provider: ProviderType::OpenAi,
            })
            .collect())
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::OpenAi
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construct_default() {
        let p = OpenAiProvider::new("sk-test".into());
        assert_eq!(p.base_url, "https://api.openai.com");
        assert_eq!(p.default_model, "gpt-4o");
        assert_eq!(p.provider_type(), ProviderType::OpenAi);
    }

    #[test]
    fn construct_with_base_url() {
        let p = OpenAiProvider::with_base_url("sk-test".into(), "http://localhost:8080".into());
        assert_eq!(p.base_url, "http://localhost:8080");
    }

    #[test]
    fn construct_with_model() {
        let p = OpenAiProvider::with_model("sk-test".into(), "gpt-4o-mini".into());
        assert_eq!(p.default_model, "gpt-4o-mini");
    }

    #[test]
    fn resolve_model_uses_default_when_empty() {
        let p = OpenAiProvider::new("sk-test".into());
        assert_eq!(p.resolve_model(""), "gpt-4o");
        assert_eq!(p.resolve_model("gpt-3.5-turbo"), "gpt-3.5-turbo");
    }
}
