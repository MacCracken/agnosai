# Adding an LLM Provider

Each LLM provider is a Rust struct implementing the `LlmProvider` trait with direct HTTP calls via `reqwest`.

## The Trait

```rust
pub trait LlmProvider: Send + Sync {
    async fn infer(&self, request: InferenceRequest) -> Result<InferenceResponse>;
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;
    fn provider_type(&self) -> ProviderType;
}
```

## Steps

### 1. Add the Provider Type

In `src/llm/mod.rs` (re-exported from hoosh), add your provider to the `ProviderType` enum:

```rust
pub enum ProviderType {
    OpenAi,
    Anthropic,
    // ...
    YourProvider,
}
```

### 2. Create the Implementation

Create `src/llm/your_provider.rs`:

```rust
use reqwest::Client;
use crate::provider::*;

pub struct YourProvider {
    client: Client,
    base_url: String,
    api_key: String,
    default_model: String,
}

impl YourProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            base_url: "https://api.yourprovider.com".to_string(),
            api_key,
            default_model: "default-model".to_string(),
        }
    }

    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        Self { base_url, ..Self::new(api_key) }
    }
}

impl LlmProvider for YourProvider {
    async fn infer(&self, request: InferenceRequest) -> Result<InferenceResponse> {
        let model = if request.model.is_empty() {
            &self.default_model
        } else {
            &request.model
        };

        // Build your provider's request format
        let body = serde_json::json!({
            "model": model,
            "messages": request.messages,
            // ... provider-specific fields
        });

        let resp = self.client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| agnosai_core::AgnosaiError::LlmProvider(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(agnosai_core::AgnosaiError::LlmProvider(text));
        }

        // Parse response into InferenceResponse
        let json: serde_json::Value = resp.json().await
            .map_err(|e| agnosai_core::AgnosaiError::LlmProvider(e.to_string()))?;

        Ok(InferenceResponse {
            content: /* extract from json */,
            model: model.to_string(),
            usage: TokenUsage { /* ... */ },
        })
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        // GET the models endpoint, or return a hardcoded list
        Ok(vec![])
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::YourProvider
    }
}
```

### 3. Register in the Module

In `src/llm/mod.rs`, add the module and re-export:

```rust
pub mod your_provider;
pub use your_provider::YourProvider;
```

### 4. OpenAI-Compatible Shortcut

If your provider uses the OpenAI API format (many do), just reuse `OpenAiProvider`:

```rust
let provider = OpenAiProvider::with_base_url(
    "your-api-key".into(),
    "https://api.yourprovider.com".into(),
);
```

This works for: DeepSeek, Mistral, Groq, LM Studio, and any vLLM/TGI deployment.
