use std::sync::Arc;

use agnosai_orchestrator::Orchestrator;
use agnosai_tools::ToolRegistry;

pub struct AppState {
    pub orchestrator: Orchestrator,
    pub tools: Arc<ToolRegistry>,
}

pub type SharedState = Arc<AppState>;
