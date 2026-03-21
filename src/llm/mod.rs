//! LLM inference via [hoosh](https://github.com/MacCracken/hoosh).
//!
//! All provider implementations, token budgeting, response caching, and
//! streaming are provided by the `hoosh` crate. This module re-exports the
//! key types and adds AgnosAI-specific task-complexity routing.

pub mod router;

// Re-export hoosh's core types so the rest of agnosai doesn't need to
// depend on hoosh directly.
pub use hoosh::budget::{TokenBudget, TokenPool};
pub use hoosh::cache::{CacheConfig, ResponseCache, cache_key};
pub use hoosh::client::HooshClient;
pub use hoosh::error::HooshError;
pub use hoosh::inference::{
    InferenceRequest, InferenceResponse, Message, ModelInfo, Role, TokenUsage,
};
pub use hoosh::provider::{LlmProvider, ProviderType};

// AgnosAI-specific task-complexity routing.
pub use router::{Complexity, ModelTier, TaskProfile, TaskType, default_model, parse_complexity};
