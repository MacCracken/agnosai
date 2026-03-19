use serde::{Deserialize, Serialize};

/// Core LLM provider trait — all providers implement this.
///
/// Each provider is a direct HTTP implementation via reqwest.
/// No Python SDKs, no litellm dependency.
#[allow(async_fn_in_trait)]
pub trait LlmProvider: Send + Sync {
    async fn infer(&self, request: InferenceRequest) -> Result<InferenceResponse>;
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;
    fn provider_type(&self) -> ProviderType;
}

pub type Result<T> = core::result::Result<T, agnosai_core::AgnosaiError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResponse {
    pub content: String,
    pub model: String,
    pub usage: TokenUsage,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub provider: ProviderType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    OpenAi,
    Anthropic,
    Gemini,
    Ollama,
    DeepSeek,
    Mistral,
    Groq,
    LmStudio,
    Hoosh,
}
