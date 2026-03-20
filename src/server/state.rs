use std::sync::Arc;

use crate::orchestrator::Orchestrator;
use crate::server::auth::AuthConfig;
use crate::server::sse::EventBus;
use crate::tools::ToolRegistry;

pub struct AppState {
    pub orchestrator: Orchestrator,
    pub tools: Arc<ToolRegistry>,
    pub auth: AuthConfig,
    pub events: EventBus,
}

pub type SharedState = Arc<AppState>;
