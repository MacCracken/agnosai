use anyhow::Result;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("agnosai=info".parse()?))
        .json()
        .init();

    tracing::info!("AgnosAI server starting");

    let app = axum::Router::new()
        .route("/health", axum::routing::get(health))
        .route("/ready", axum::routing::get(ready));

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> &'static str {
    "ok"
}

async fn ready() -> &'static str {
    // TODO: Check orchestrator + provider health
    "ok"
}
