use std::sync::Arc;

use crate::llm::AuditChain;
use crate::orchestrator::Orchestrator;
use crate::server::auth::AuthConfig;
use crate::server::sse::EventBus;
use crate::tools::ToolRegistry;

/// Shared application state accessible from all route handlers.
pub struct AppState {
    /// Crew orchestrator.
    pub orchestrator: Orchestrator,
    /// Registered tool instances.
    pub tools: Arc<ToolRegistry>,
    /// Authentication configuration.
    pub auth: AuthConfig,
    /// SSE event bus for crew streaming.
    pub events: EventBus,
    /// Shared HTTP client for outbound requests (A2A callbacks, etc.).
    pub http_client: reqwest::Client,
    /// Cryptographic audit chain for tamper-proof event logging.
    pub audit: Arc<AuditChain>,
}

/// Thread-safe shared reference to application state.
pub type SharedState = Arc<AppState>;
