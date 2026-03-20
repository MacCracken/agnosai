use std::sync::Arc;

use agnosai::orchestrator::Orchestrator;
use agnosai::server::sse::EventBus;
use agnosai::tools::ToolRegistry;
use agnosai::tools::builtin::echo::EchoTool;
use agnosai::tools::builtin::json_transform::JsonTransformTool;
use agnosai::tools::builtin::load_testing::LoadTestingTool;
use agnosai::tools::builtin::security_audit::SecurityAuditTool;
use tracing_subscriber::EnvFilter;

use agnosai::server::{AppState, SharedState, router};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("agnosai=info".parse()?))
        .json()
        .init();

    tracing::info!("AgnosAI server starting");

    // Initialise shared state.
    let events = EventBus::new();
    let orchestrator = Orchestrator::new(Default::default())
        .await?
        .with_events(events.clone());
    let tools = Arc::new(ToolRegistry::new());
    tools.register(Arc::new(EchoTool));
    tools.register(Arc::new(JsonTransformTool));
    tools.register(Arc::new(LoadTestingTool));
    tools.register(Arc::new(SecurityAuditTool));

    let state: SharedState = Arc::new(AppState {
        orchestrator,
        tools,
        auth: Default::default(),
        events,
    });

    let app = router(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
