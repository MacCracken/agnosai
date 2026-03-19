//! LLM provider abstraction with native HTTP implementations.
//!
//! Every provider is a direct HTTP client via `reqwest` — no Python SDKs,
//! no litellm dependency. Includes model routing, health tracking, response
//! caching, token budgeting, and rate limiting.
//!
//! # Providers
//!
//! | Provider | Type |
//! |----------|------|
//! | OpenAI | Direct REST |
//! | Anthropic | Direct REST |
//! | Ollama | Direct REST |
//! | DeepSeek, Mistral, Groq, LM Studio, hoosh | OpenAI-compatible wrappers |

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
pub use cache::{ResponseCache, cache_key};
