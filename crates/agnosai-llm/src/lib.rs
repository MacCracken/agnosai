pub mod budget;
pub mod cache;
pub mod health;
pub mod provider;
pub mod rate_limiter;
pub mod router;

pub mod providers;

// Re-export key types for ergonomic use.
pub use health::ProviderHealth;
pub use provider::{
    ChatMessage, InferenceRequest, InferenceResponse, LlmProvider, ModelInfo, ProviderType,
    TokenUsage,
};
pub use rate_limiter::RateLimiter;
pub use router::{Complexity, ModelTier, TaskProfile, TaskType};

// Re-export provider implementations.
pub use providers::anthropic::AnthropicProvider;
pub use providers::deepseek::DeepSeekProvider;
pub use providers::groq::GroqProvider;
pub use providers::hoosh::HooshProvider;
pub use providers::lmstudio::LmStudioProvider;
pub use providers::mistral::MistralProvider;
pub use providers::ollama::OllamaProvider;
pub use providers::openai::OpenAiProvider;

// Re-export cache and budget types.
pub use budget::{BudgetExceeded, BudgetSummary, TokenBudget};
pub use cache::{cache_key, ResponseCache};
