//! OpenAI-compatible provider via direct HTTP (reqwest).
//!
//! Works with OpenAI, Azure OpenAI, and any API-compatible endpoint
//! (e.g. vLLM, LiteLLM proxy) by setting a custom `base_url`.

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::llm::provider::{
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

    /// Create a provider with custom base URL and default model.
    pub fn with_base_url_and_model(api_key: String, base_url: String, model: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
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

    /// Base URL (for testing wrapper providers).
    #[cfg(test)]
    pub(crate) fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Default model (for testing wrapper providers).
    #[cfg(test)]
    pub(crate) fn default_model(&self) -> &str {
        &self.default_model
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
            .map_err(|e| crate::core::AgnosaiError::LlmProvider(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".to_string());
            let detail = serde_json::from_str::<OaiErrorResponse>(&text)
                .map(|e| e.error.message)
                .unwrap_or(text);
            return Err(crate::core::AgnosaiError::LlmProvider(format!(
                "OpenAI HTTP {status}: {detail}"
            )));
        }

        let oai: OaiChatResponse = resp
            .json()
            .await
            .map_err(|e| crate::core::AgnosaiError::LlmProvider(e.to_string()))?;

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
            .map_err(|e| crate::core::AgnosaiError::LlmProvider(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(crate::core::AgnosaiError::LlmProvider(format!(
                "OpenAI HTTP {status}: {text}"
            )));
        }

        let oai: OaiModelsResponse = resp
            .json()
            .await
            .map_err(|e| crate::core::AgnosaiError::LlmProvider(e.to_string()))?;

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
    use crate::llm::provider::ChatMessage;

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

    // ── Request body serialization tests ────────────────────────────

    #[test]
    fn request_body_json_structure() {
        let body = OaiChatRequest {
            model: "gpt-4o".to_string(),
            messages: vec![
                OaiMessage {
                    role: "system".to_string(),
                    content: "You are helpful.".to_string(),
                },
                OaiMessage {
                    role: "user".to_string(),
                    content: "Hello".to_string(),
                },
            ],
            temperature: Some(0.7),
            max_tokens: Some(1024),
            stream: false,
        };

        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["model"], "gpt-4o");
        assert_eq!(json["stream"], false);
        assert_eq!(json["temperature"], 0.7);
        assert_eq!(json["max_tokens"], 1024);

        let msgs = json["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0]["role"], "system");
        assert_eq!(msgs[0]["content"], "You are helpful.");
        assert_eq!(msgs[1]["role"], "user");
        assert_eq!(msgs[1]["content"], "Hello");
    }

    #[test]
    fn request_body_omits_none_fields() {
        let body = OaiChatRequest {
            model: "gpt-4o".to_string(),
            messages: vec![],
            temperature: None,
            max_tokens: None,
            stream: false,
        };

        let json = serde_json::to_value(&body).unwrap();
        assert!(json.get("temperature").is_none());
        assert!(json.get("max_tokens").is_none());
        // stream and model are always present
        assert_eq!(json["stream"], false);
        assert_eq!(json["model"], "gpt-4o");
    }

    #[test]
    fn request_body_maps_from_inference_request() {
        let request = InferenceRequest {
            model: "".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Hi".to_string(),
            }],
            temperature: Some(0.5),
            max_tokens: Some(256),
            stream: false,
        };

        let p = OpenAiProvider::new("sk-test".into());
        let model = p.resolve_model(&request.model);

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
            stream: false,
        };

        let json = serde_json::to_value(&body).unwrap();
        // Empty model should resolve to default
        assert_eq!(json["model"], "gpt-4o");
        assert_eq!(json["messages"][0]["content"], "Hi");
        assert_eq!(json["temperature"], 0.5);
        assert_eq!(json["max_tokens"], 256);
    }

    // ── Response parsing tests ──────────────────────────────────────

    #[test]
    fn parse_chat_response() {
        let json = serde_json::json!({
            "id": "chatcmpl-abc123",
            "object": "chat.completion",
            "model": "gpt-4o-2025-01-01",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello! How can I help you?"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 8,
                "total_tokens": 18
            }
        });

        let oai: OaiChatResponse = serde_json::from_value(json).unwrap();

        assert_eq!(oai.model, "gpt-4o-2025-01-01");
        assert_eq!(oai.choices.len(), 1);
        assert_eq!(oai.choices[0].message.content, "Hello! How can I help you?");
        assert_eq!(oai.choices[0].message.role, "assistant");

        let usage = oai.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 8);
        assert_eq!(usage.total_tokens, 18);
    }

    #[test]
    fn parse_chat_response_no_usage() {
        let json = serde_json::json!({
            "model": "gpt-4o",
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Hi"
                }
            }]
        });

        let oai: OaiChatResponse = serde_json::from_value(json).unwrap();
        assert!(oai.usage.is_none());
        assert_eq!(oai.choices[0].message.content, "Hi");
    }

    #[test]
    fn parse_chat_response_empty_choices() {
        let json = serde_json::json!({
            "model": "gpt-4o",
            "choices": []
        });

        let oai: OaiChatResponse = serde_json::from_value(json).unwrap();
        // The provider code uses .first().unwrap_or_default() — empty choices yields ""
        let content = oai
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();
        assert_eq!(content, "");
    }

    #[test]
    fn parse_chat_response_to_inference_response() {
        let json = serde_json::json!({
            "model": "gpt-4o",
            "choices": [{
                "message": { "role": "assistant", "content": "Answer" }
            }],
            "usage": {
                "prompt_tokens": 5,
                "completion_tokens": 3,
                "total_tokens": 8
            }
        });

        let oai: OaiChatResponse = serde_json::from_value(json).unwrap();

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

        let resp = InferenceResponse {
            content,
            model: oai.model,
            usage,
        };

        assert_eq!(resp.content, "Answer");
        assert_eq!(resp.model, "gpt-4o");
        assert_eq!(resp.usage.prompt_tokens, 5);
        assert_eq!(resp.usage.completion_tokens, 3);
        assert_eq!(resp.usage.total_tokens, 8);
    }

    #[test]
    fn parse_error_response() {
        let json = serde_json::json!({
            "error": {
                "message": "Invalid API key provided",
                "type": "invalid_request_error",
                "code": "invalid_api_key"
            }
        });

        let err: OaiErrorResponse = serde_json::from_value(json).unwrap();
        assert_eq!(err.error.message, "Invalid API key provided");
    }

    #[test]
    fn parse_error_response_fallback_to_raw_text() {
        // When the body is not valid JSON for OaiErrorResponse, code falls back to raw text
        let raw = "Service Unavailable";
        let result = serde_json::from_str::<OaiErrorResponse>(raw);
        assert!(result.is_err());
        // The provider code uses .unwrap_or(text) — so raw text is the detail
    }

    #[test]
    fn parse_models_response() {
        let json = serde_json::json!({
            "data": [
                { "id": "gpt-4o" },
                { "id": "gpt-4o-mini" },
                { "id": "gpt-3.5-turbo" }
            ]
        });

        let models: OaiModelsResponse = serde_json::from_value(json).unwrap();
        assert_eq!(models.data.len(), 3);
        assert_eq!(models.data[0].id, "gpt-4o");
        assert_eq!(models.data[1].id, "gpt-4o-mini");
        assert_eq!(models.data[2].id, "gpt-3.5-turbo");

        // Convert to ModelInfo the same way the provider does
        let infos: Vec<ModelInfo> = models
            .data
            .into_iter()
            .map(|m| ModelInfo {
                id: m.id.clone(),
                name: m.id,
                provider: ProviderType::OpenAi,
            })
            .collect();
        assert_eq!(infos.len(), 3);
        assert_eq!(infos[0].name, "gpt-4o");
        assert_eq!(infos[0].provider, ProviderType::OpenAi);
    }

    #[test]
    fn parse_models_response_empty() {
        let json = serde_json::json!({ "data": [] });
        let models: OaiModelsResponse = serde_json::from_value(json).unwrap();
        assert!(models.data.is_empty());
    }
}
