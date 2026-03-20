use std::sync::Arc;

use crate::orchestrator::Orchestrator;
use crate::tools::ToolRegistry;

pub struct AppState {
    pub orchestrator: Orchestrator,
    pub tools: Arc<ToolRegistry>,
}

pub type SharedState = Arc<AppState>;
