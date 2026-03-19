//! Mistral provider — thin wrapper over the OpenAI-compatible API.

use crate::provider::{
    InferenceRequest, InferenceResponse, LlmProvider, ModelInfo, ProviderType, Result,
};
use crate::providers::openai::OpenAiProvider;

/// Mistral LLM provider (OpenAI-compatible).
pub struct MistralProvider(OpenAiProvider);

impl MistralProvider {
    pub fn new(api_key: String) -> Self {
        Self(OpenAiProvider::with_base_url_and_model(
            api_key,
            "https://api.mistral.ai".to_string(),
            "mistral-large-latest".to_string(),
        ))
    }
}

impl LlmProvider for MistralProvider {
    async fn infer(&self, request: InferenceRequest) -> Result<InferenceResponse> {
        self.0.infer(request).await
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        self.0.list_models().await
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Mistral
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_type() {
        let p = MistralProvider::new("sk-test".into());
        assert_eq!(p.provider_type(), ProviderType::Mistral);
    }

    #[test]
    fn base_url_is_mistral() {
        let p = MistralProvider::new("sk-test".into());
        assert_eq!(p.0.base_url(), "https://api.mistral.ai");
    }

    #[test]
    fn default_model_is_mistral_large() {
        let p = MistralProvider::new("sk-test".into());
        assert_eq!(p.0.default_model(), "mistral-large-latest");
    }
}
