//! Anthropic provider via direct HTTP (reqwest).
//!
//! Uses the Anthropic Messages API (`/v1/messages`) with `x-api-key` auth.

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::provider::{
    InferenceRequest, InferenceResponse, LlmProvider, ModelInfo, ProviderType, Result, TokenUsage,
};

const API_BASE: &str = "https://api.anthropic.com";
const API_VERSION: &str = "2023-06-01";
const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";

/// Anthropic LLM provider (Claude family).
pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    default_model: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            default_model: DEFAULT_MODEL.to_string(),
        }
    }

    pub fn with_model(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
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

// ── Anthropic API types ─────────────────────────────────────────────

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
}

#[derive(Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContentBlock>,
    model: String,
    usage: AnthropicUsage,
}

#[derive(Deserialize)]
struct AnthropicContentBlock {
    text: String,
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Deserialize)]
struct AnthropicErrorResponse {
    error: AnthropicErrorDetail,
}

#[derive(Deserialize)]
struct AnthropicErrorDetail {
    message: String,
}

// ── LlmProvider impl ───────────────────────────────────────────────

impl LlmProvider for AnthropicProvider {
    async fn infer(&self, request: InferenceRequest) -> Result<InferenceResponse> {
        let model = self.resolve_model(&request.model);
        let url = format!("{API_BASE}/v1/messages");

        // Separate system messages from user/assistant messages.
        let mut system_parts: Vec<String> = Vec::new();
        let mut messages: Vec<AnthropicMessage> = Vec::new();

        for msg in &request.messages {
            if msg.role == "system" {
                system_parts.push(msg.content.clone());
            } else {
                messages.push(AnthropicMessage {
                    role: msg.role.clone(),
                    content: msg.content.clone(),
                });
            }
        }

        let system = if system_parts.is_empty() {
            None
        } else {
            Some(system_parts.join("\n\n"))
        };

        let max_tokens = request.max_tokens.unwrap_or(4096);

        let body = AnthropicRequest {
            model,
            messages,
            max_tokens,
            system,
            temperature: request.temperature,
        };

        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| agnosai_core::AgnosaiError::LlmProvider(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            let detail = serde_json::from_str::<AnthropicErrorResponse>(&text)
                .map(|e| e.error.message)
                .unwrap_or(text);
            return Err(agnosai_core::AgnosaiError::LlmProvider(format!(
                "Anthropic HTTP {status}: {detail}"
            )));
        }

        let ar: AnthropicResponse = resp
            .json()
            .await
            .map_err(|e| agnosai_core::AgnosaiError::LlmProvider(e.to_string()))?;

        let content = ar
            .content
            .into_iter()
            .map(|b| b.text)
            .collect::<Vec<_>>()
            .join("");

        let total = ar.usage.input_tokens + ar.usage.output_tokens;

        Ok(InferenceResponse {
            content,
            model: ar.model,
            usage: TokenUsage {
                prompt_tokens: ar.usage.input_tokens,
                completion_tokens: ar.usage.output_tokens,
                total_tokens: total,
            },
        })
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        // The Anthropic API does not have a list-models endpoint.
        // Return the well-known model family.
        Ok(vec![
            ModelInfo {
                id: "claude-sonnet-4-20250514".into(),
                name: "Claude Sonnet 4".into(),
                provider: ProviderType::Anthropic,
            },
            ModelInfo {
                id: "claude-opus-4-20250514".into(),
                name: "Claude Opus 4".into(),
                provider: ProviderType::Anthropic,
            },
            ModelInfo {
                id: "claude-3-5-haiku-20241022".into(),
                name: "Claude 3.5 Haiku".into(),
                provider: ProviderType::Anthropic,
            },
        ])
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Anthropic
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::ChatMessage;

    #[test]
    fn construct_default() {
        let p = AnthropicProvider::new("sk-ant-test".into());
        assert_eq!(p.default_model, DEFAULT_MODEL);
        assert_eq!(p.provider_type(), ProviderType::Anthropic);
    }

    #[test]
    fn construct_with_model() {
        let p = AnthropicProvider::with_model("sk-ant-test".into(), "claude-opus-4-20250514".into());
        assert_eq!(p.default_model, "claude-opus-4-20250514");
    }

    #[test]
    fn system_message_extraction() {
        // Verify the system-message separation logic compiles and works
        let messages = vec![
            ChatMessage {
                role: "system".into(),
                content: "You are helpful.".into(),
            },
            ChatMessage {
                role: "user".into(),
                content: "Hello".into(),
            },
        ];

        let mut system_parts: Vec<String> = Vec::new();
        let mut api_messages: Vec<AnthropicMessage> = Vec::new();
        for msg in &messages {
            if msg.role == "system" {
                system_parts.push(msg.content.clone());
            } else {
                api_messages.push(AnthropicMessage {
                    role: msg.role.clone(),
                    content: msg.content.clone(),
                });
            }
        }

        assert_eq!(system_parts.len(), 1);
        assert_eq!(api_messages.len(), 1);
        assert_eq!(api_messages[0].role, "user");
    }
}
