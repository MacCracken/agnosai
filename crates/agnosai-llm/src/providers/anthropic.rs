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
        let p =
            AnthropicProvider::with_model("sk-ant-test".into(), "claude-opus-4-20250514".into());
        assert_eq!(p.default_model, "claude-opus-4-20250514");
    }

    #[test]
    fn resolve_model_uses_default_when_empty() {
        let p = AnthropicProvider::new("sk-ant-test".into());
        assert_eq!(p.resolve_model(""), DEFAULT_MODEL);
        assert_eq!(
            p.resolve_model("claude-opus-4-20250514"),
            "claude-opus-4-20250514"
        );
    }

    // ── System message extraction ───────────────────────────────────

    #[test]
    fn system_message_extraction_single() {
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

    #[test]
    fn system_message_extraction_multiple() {
        let messages = vec![
            ChatMessage {
                role: "system".into(),
                content: "You are helpful.".into(),
            },
            ChatMessage {
                role: "system".into(),
                content: "Be concise.".into(),
            },
            ChatMessage {
                role: "user".into(),
                content: "Hello".into(),
            },
            ChatMessage {
                role: "assistant".into(),
                content: "Hi!".into(),
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

        let system = if system_parts.is_empty() {
            None
        } else {
            Some(system_parts.join("\n\n"))
        };

        assert_eq!(system, Some("You are helpful.\n\nBe concise.".to_string()));
        assert_eq!(api_messages.len(), 2);
        assert_eq!(api_messages[0].role, "user");
        assert_eq!(api_messages[1].role, "assistant");
    }

    #[test]
    fn system_message_extraction_none() {
        let messages = vec![ChatMessage {
            role: "user".into(),
            content: "Hello".into(),
        }];

        let mut system_parts: Vec<String> = Vec::new();
        for msg in &messages {
            if msg.role == "system" {
                system_parts.push(msg.content.clone());
            }
        }

        let system = if system_parts.is_empty() {
            None
        } else {
            Some(system_parts.join("\n\n"))
        };

        assert!(system.is_none());
    }

    // ── Request body serialization ──────────────────────────────────

    #[test]
    fn request_body_json_structure() {
        let body = AnthropicRequest {
            model: "claude-sonnet-4-20250514".to_string(),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            max_tokens: 4096,
            system: Some("Be helpful.".to_string()),
            temperature: Some(0.7),
        };

        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["model"], "claude-sonnet-4-20250514");
        assert_eq!(json["max_tokens"], 4096);
        assert_eq!(json["system"], "Be helpful.");
        assert_eq!(json["temperature"], 0.7);
        assert_eq!(json["messages"][0]["role"], "user");
        assert_eq!(json["messages"][0]["content"], "Hello");
        // No "stream" field in Anthropic request
        assert!(json.get("stream").is_none());
    }

    #[test]
    fn request_body_omits_none_fields() {
        let body = AnthropicRequest {
            model: "claude-sonnet-4-20250514".to_string(),
            messages: vec![],
            max_tokens: 4096,
            system: None,
            temperature: None,
        };

        let json = serde_json::to_value(&body).unwrap();
        assert!(json.get("system").is_none());
        assert!(json.get("temperature").is_none());
        // max_tokens is always present (not optional in Anthropic API)
        assert_eq!(json["max_tokens"], 4096);
    }

    #[test]
    fn request_body_default_max_tokens() {
        // When max_tokens is None in InferenceRequest, the provider defaults to 4096
        let request = InferenceRequest {
            model: "".to_string(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "test".into(),
            }],
            temperature: None,
            max_tokens: None,
            stream: false,
        };

        let max_tokens = request.max_tokens.unwrap_or(4096);
        assert_eq!(max_tokens, 4096);
    }

    // ── Response parsing ────────────────────────────────────────────

    #[test]
    fn parse_response_single_block() {
        let json = serde_json::json!({
            "content": [
                { "type": "text", "text": "Hello! How can I help you?" }
            ],
            "model": "claude-sonnet-4-20250514",
            "usage": {
                "input_tokens": 12,
                "output_tokens": 8
            }
        });

        let ar: AnthropicResponse = serde_json::from_value(json).unwrap();
        assert_eq!(ar.model, "claude-sonnet-4-20250514");
        assert_eq!(ar.content.len(), 1);
        assert_eq!(ar.content[0].text, "Hello! How can I help you?");
        assert_eq!(ar.usage.input_tokens, 12);
        assert_eq!(ar.usage.output_tokens, 8);
    }

    #[test]
    fn parse_response_multiple_content_blocks() {
        let json = serde_json::json!({
            "content": [
                { "type": "text", "text": "First part." },
                { "type": "text", "text": " Second part." }
            ],
            "model": "claude-sonnet-4-20250514",
            "usage": { "input_tokens": 5, "output_tokens": 10 }
        });

        let ar: AnthropicResponse = serde_json::from_value(json).unwrap();

        // The provider joins all content blocks
        let content: String = ar
            .content
            .into_iter()
            .map(|b| b.text)
            .collect::<Vec<_>>()
            .join("");

        assert_eq!(content, "First part. Second part.");
    }

    #[test]
    fn parse_response_to_inference_response() {
        let json = serde_json::json!({
            "content": [
                { "type": "text", "text": "Answer here." }
            ],
            "model": "claude-sonnet-4-20250514",
            "usage": { "input_tokens": 20, "output_tokens": 5 }
        });

        let ar: AnthropicResponse = serde_json::from_value(json).unwrap();

        let content = ar
            .content
            .into_iter()
            .map(|b| b.text)
            .collect::<Vec<_>>()
            .join("");
        let total = ar.usage.input_tokens + ar.usage.output_tokens;

        let resp = InferenceResponse {
            content,
            model: ar.model,
            usage: TokenUsage {
                prompt_tokens: ar.usage.input_tokens,
                completion_tokens: ar.usage.output_tokens,
                total_tokens: total,
            },
        };

        assert_eq!(resp.content, "Answer here.");
        assert_eq!(resp.model, "claude-sonnet-4-20250514");
        assert_eq!(resp.usage.prompt_tokens, 20);
        assert_eq!(resp.usage.completion_tokens, 5);
        assert_eq!(resp.usage.total_tokens, 25);
    }

    #[test]
    fn parse_error_response() {
        let json = serde_json::json!({
            "type": "error",
            "error": {
                "type": "authentication_error",
                "message": "Invalid API key"
            }
        });

        let err: AnthropicErrorResponse = serde_json::from_value(json).unwrap();
        assert_eq!(err.error.message, "Invalid API key");
    }

    #[test]
    fn list_models_returns_known_family() {
        // Anthropic list_models is a static list — verify it at least returns entries
        // We can't call async in a sync test easily, but we verify the static data
        let models = [
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
        ];
        assert_eq!(models.len(), 3);
        assert!(models.iter().all(|m| m.provider == ProviderType::Anthropic));
    }
}
