use std::sync::Arc;

use agnosai::orchestrator::Orchestrator;
use agnosai::tools::ToolRegistry;
use agnosai::tools::builtin::echo::EchoTool;
use agnosai::tools::builtin::json_transform::JsonTransformTool;
use anyhow::Result;
use tracing_subscriber::EnvFilter;

use agnosai::server::{AppState, SharedState, router};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("agnosai=info".parse()?))
        .json()
        .init();

    tracing::info!("AgnosAI server starting");

    // Initialise shared state.
    let orchestrator = Orchestrator::new(Default::default()).await?;
    let tools = Arc::new(ToolRegistry::new());
    tools.register(Arc::new(EchoTool));
    tools.register(Arc::new(JsonTransformTool));

    let state: SharedState = Arc::new(AppState {
        orchestrator,
        tools,
        auth: Default::default(),
    });

    let app = router(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
