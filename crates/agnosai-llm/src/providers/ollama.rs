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
    use crate::provider::ChatMessage;

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

    // ── Request body serialization ──────────────────────────────────

    #[test]
    fn request_body_json_structure() {
        let body = OllamaChatRequest {
            model: "llama3".to_string(),
            messages: vec![OllamaMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            stream: false,
            options: Some(OllamaOptions {
                temperature: Some(0.8),
                num_predict: Some(512),
            }),
        };

        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["model"], "llama3");
        assert_eq!(json["stream"], false);
        assert_eq!(json["options"]["temperature"], 0.8);
        assert_eq!(json["options"]["num_predict"], 512);
        assert_eq!(json["messages"][0]["role"], "user");
        assert_eq!(json["messages"][0]["content"], "Hello");
    }

    #[test]
    fn request_body_no_options_when_none() {
        let body = OllamaChatRequest {
            model: "llama3".to_string(),
            messages: vec![],
            stream: false,
            options: None,
        };

        let json = serde_json::to_value(&body).unwrap();
        assert!(json.get("options").is_none());
    }

    #[test]
    fn request_body_options_omits_none_fields() {
        let body = OllamaChatRequest {
            model: "llama3".to_string(),
            messages: vec![],
            stream: false,
            options: Some(OllamaOptions {
                temperature: Some(0.5),
                num_predict: None,
            }),
        };

        let json = serde_json::to_value(&body).unwrap();
        let opts = &json["options"];
        assert_eq!(opts["temperature"], 0.5);
        assert!(opts.get("num_predict").is_none());
    }

    #[test]
    fn request_options_built_from_inference_request() {
        // When both temperature and max_tokens are present
        let request = InferenceRequest {
            model: "llama3".to_string(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "test".into(),
            }],
            temperature: Some(0.9),
            max_tokens: Some(100),
            stream: false,
        };

        let options = if request.temperature.is_some() || request.max_tokens.is_some() {
            Some(OllamaOptions {
                temperature: request.temperature,
                num_predict: request.max_tokens,
            })
        } else {
            None
        };

        assert!(options.is_some());
        let opts = options.unwrap();
        assert_eq!(opts.temperature, Some(0.9));
        assert_eq!(opts.num_predict, Some(100));
    }

    #[test]
    fn request_options_none_when_no_params() {
        let request = InferenceRequest {
            model: "llama3".to_string(),
            messages: vec![],
            temperature: None,
            max_tokens: None,
            stream: false,
        };

        let options = if request.temperature.is_some() || request.max_tokens.is_some() {
            Some(OllamaOptions {
                temperature: request.temperature,
                num_predict: request.max_tokens,
            })
        } else {
            None
        };

        assert!(options.is_none());
    }

    // ── Response parsing ────────────────────────────────────────────

    #[test]
    fn parse_chat_response() {
        let json = serde_json::json!({
            "model": "llama3:latest",
            "message": {
                "role": "assistant",
                "content": "Hello! I'm Llama."
            },
            "prompt_eval_count": 15,
            "eval_count": 10
        });

        let ol: OllamaChatResponse = serde_json::from_value(json).unwrap();
        assert_eq!(ol.model, "llama3:latest");
        assert_eq!(ol.message.content, "Hello! I'm Llama.");
        assert_eq!(ol.message.role, "assistant");
        assert_eq!(ol.prompt_eval_count, Some(15));
        assert_eq!(ol.eval_count, Some(10));
    }

    #[test]
    fn parse_chat_response_missing_token_counts() {
        let json = serde_json::json!({
            "model": "llama3",
            "message": {
                "role": "assistant",
                "content": "Hi"
            }
        });

        let ol: OllamaChatResponse = serde_json::from_value(json).unwrap();
        assert!(ol.prompt_eval_count.is_none());
        assert!(ol.eval_count.is_none());

        // Provider uses unwrap_or(0)
        let prompt_tokens = ol.prompt_eval_count.unwrap_or(0);
        let completion_tokens = ol.eval_count.unwrap_or(0);
        assert_eq!(prompt_tokens, 0);
        assert_eq!(completion_tokens, 0);
    }

    #[test]
    fn parse_chat_response_to_inference_response() {
        let json = serde_json::json!({
            "model": "llama3:latest",
            "message": {
                "role": "assistant",
                "content": "The answer is 42."
            },
            "prompt_eval_count": 20,
            "eval_count": 5
        });

        let ol: OllamaChatResponse = serde_json::from_value(json).unwrap();

        let prompt_tokens = ol.prompt_eval_count.unwrap_or(0);
        let completion_tokens = ol.eval_count.unwrap_or(0);

        let resp = InferenceResponse {
            content: ol.message.content,
            model: ol.model,
            usage: TokenUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            },
        };

        assert_eq!(resp.content, "The answer is 42.");
        assert_eq!(resp.model, "llama3:latest");
        assert_eq!(resp.usage.prompt_tokens, 20);
        assert_eq!(resp.usage.completion_tokens, 5);
        assert_eq!(resp.usage.total_tokens, 25);
    }

    #[test]
    fn parse_tags_response() {
        let json = serde_json::json!({
            "models": [
                { "name": "llama3:latest" },
                { "name": "mistral:7b" },
                { "name": "codellama:13b" }
            ]
        });

        let tags: OllamaTagsResponse = serde_json::from_value(json).unwrap();
        assert_eq!(tags.models.len(), 3);

        let infos: Vec<ModelInfo> = tags
            .models
            .into_iter()
            .map(|m| ModelInfo {
                id: m.name.clone(),
                name: m.name,
                provider: ProviderType::Ollama,
            })
            .collect();

        assert_eq!(infos[0].id, "llama3:latest");
        assert_eq!(infos[2].name, "codellama:13b");
        assert!(infos.iter().all(|m| m.provider == ProviderType::Ollama));
    }

    #[test]
    fn parse_tags_response_empty() {
        let json = serde_json::json!({ "models": [] });
        let tags: OllamaTagsResponse = serde_json::from_value(json).unwrap();
        assert!(tags.models.is_empty());
    }
}
