//! LM Studio provider — thin wrapper over the OpenAI-compatible API.

use crate::provider::{InferenceRequest, InferenceResponse, LlmProvider, ModelInfo, ProviderType, Result};
use crate::providers::openai::OpenAiProvider;

/// LM Studio LLM provider (local, OpenAI-compatible).
pub struct LmStudioProvider(OpenAiProvider);

impl LmStudioProvider {
    pub fn new() -> Self {
        Self(OpenAiProvider::with_base_url_and_model(
            String::new(),
            "http://localhost:1234".to_string(),
            "default".to_string(),
        ))
    }
}

impl Default for LmStudioProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmProvider for LmStudioProvider {
    async fn infer(&self, request: InferenceRequest) -> Result<InferenceResponse> {
        self.0.infer(request).await
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        self.0.list_models().await
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::LmStudio
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_type() {
        let p = LmStudioProvider::new();
        assert_eq!(p.provider_type(), ProviderType::LmStudio);
    }
}
