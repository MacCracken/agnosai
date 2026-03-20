use std::sync::Arc;

use crate::orchestrator::Orchestrator;
use crate::server::auth::AuthConfig;
use crate::tools::ToolRegistry;

pub struct AppState {
    pub orchestrator: Orchestrator,
    pub tools: Arc<ToolRegistry>,
    pub auth: AuthConfig,
}

pub type SharedState = Arc<AppState>;
