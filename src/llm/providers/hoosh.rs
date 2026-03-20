//! Hoosh gateway provider — thin wrapper over the OpenAI-compatible API.

use crate::llm::provider::{
    InferenceRequest, InferenceResponse, LlmProvider, ModelInfo, ProviderType, Result,
};
use crate::llm::providers::openai::OpenAiProvider;

/// Hoosh LLM gateway provider (OpenAI-compatible).
pub struct HooshProvider(OpenAiProvider);

impl HooshProvider {
    pub fn new() -> Self {
        Self(OpenAiProvider::with_base_url_and_model(
            String::new(),
            "http://localhost:8088".to_string(),
            "default".to_string(),
        ))
    }

    pub fn with_base_url(base_url: String) -> Self {
        Self(OpenAiProvider::with_base_url_and_model(
            String::new(),
            base_url,
            "default".to_string(),
        ))
    }
}

impl Default for HooshProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmProvider for HooshProvider {
    async fn infer(&self, request: InferenceRequest) -> Result<InferenceResponse> {
        self.0.infer(request).await
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        self.0.list_models().await
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Hoosh
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_type_default() {
        let p = HooshProvider::new();
        assert_eq!(p.provider_type(), ProviderType::Hoosh);
    }

    #[test]
    fn provider_type_custom_url() {
        let p = HooshProvider::with_base_url("http://remote:9090".into());
        assert_eq!(p.provider_type(), ProviderType::Hoosh);
    }

    #[test]
    fn base_url_default() {
        let p = HooshProvider::new();
        assert_eq!(p.0.base_url(), "http://localhost:8088");
    }

    #[test]
    fn base_url_custom() {
        let p = HooshProvider::with_base_url("http://gateway:8088".into());
        assert_eq!(p.0.base_url(), "http://gateway:8088");
    }

    #[test]
    fn default_model_is_default() {
        let p = HooshProvider::new();
        assert_eq!(p.0.default_model(), "default");
    }

    #[test]
    fn default_trait_matches_new() {
        let p1 = HooshProvider::new();
        let p2 = HooshProvider::default();
        assert_eq!(p1.0.base_url(), p2.0.base_url());
        assert_eq!(p1.0.default_model(), p2.0.default_model());
    }
}
