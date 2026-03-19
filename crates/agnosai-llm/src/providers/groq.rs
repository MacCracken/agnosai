//! Groq provider — thin wrapper over the OpenAI-compatible API.

use crate::provider::{InferenceRequest, InferenceResponse, LlmProvider, ModelInfo, ProviderType, Result};
use crate::providers::openai::OpenAiProvider;

/// Groq LLM provider (OpenAI-compatible).
pub struct GroqProvider(OpenAiProvider);

impl GroqProvider {
    pub fn new(api_key: String) -> Self {
        Self(OpenAiProvider::with_base_url_and_model(
            api_key,
            "https://api.groq.com/openai".to_string(),
            "llama-3.3-70b-versatile".to_string(),
        ))
    }
}

impl LlmProvider for GroqProvider {
    async fn infer(&self, request: InferenceRequest) -> Result<InferenceResponse> {
        self.0.infer(request).await
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        self.0.list_models().await
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Groq
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_type() {
        let p = GroqProvider::new("sk-test".into());
        assert_eq!(p.provider_type(), ProviderType::Groq);
    }
}
