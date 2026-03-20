//! DeepSeek provider — thin wrapper over the OpenAI-compatible API.

use crate::llm::provider::{
    InferenceRequest, InferenceResponse, LlmProvider, ModelInfo, ProviderType, Result,
};
use crate::llm::providers::openai::OpenAiProvider;

/// DeepSeek LLM provider (OpenAI-compatible).
pub struct DeepSeekProvider(OpenAiProvider);

impl DeepSeekProvider {
    pub fn new(api_key: String) -> Self {
        Self(OpenAiProvider::with_base_url_and_model(
            api_key,
            "https://api.deepseek.com".to_string(),
            "deepseek-chat".to_string(),
        ))
    }
}

impl LlmProvider for DeepSeekProvider {
    async fn infer(&self, request: InferenceRequest) -> Result<InferenceResponse> {
        self.0.infer(request).await
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        self.0.list_models().await
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::DeepSeek
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_type() {
        let p = DeepSeekProvider::new("sk-test".into());
        assert_eq!(p.provider_type(), ProviderType::DeepSeek);
    }

    #[test]
    fn base_url_is_deepseek() {
        let p = DeepSeekProvider::new("sk-test".into());
        assert_eq!(p.0.base_url(), "https://api.deepseek.com");
    }

    #[test]
    fn default_model_is_deepseek_chat() {
        let p = DeepSeekProvider::new("sk-test".into());
        assert_eq!(p.0.default_model(), "deepseek-chat");
    }
}
