use std::sync::Arc;

use agnosai::orchestrator::Orchestrator;
use agnosai::server::auth::{AuthConfig, JwtConfig};
use agnosai::server::sse::EventBus;
use agnosai::tools::ToolRegistry;
use agnosai::tools::builtin::echo::EchoTool;
use agnosai::tools::builtin::json_transform::JsonTransformTool;
use agnosai::tools::builtin::load_testing::LoadTestingTool;
use agnosai::tools::builtin::security_audit::SecurityAuditTool;
#[cfg(not(feature = "otel"))]
use tracing_subscriber::EnvFilter;

use agnosai::server::{AppState, SharedState, router};

fn load_auth_config() -> AuthConfig {
    let enabled = std::env::var("AGNOSAI_AUTH_ENABLED")
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or(false);

    let secret = std::env::var("AGNOSAI_AUTH_SECRET").unwrap_or_default();

    let jwt = std::env::var("AGNOSAI_JWT_PUBLIC_KEY")
        .ok()
        .filter(|k| !k.is_empty())
        .map(|key| {
            let mut cfg = JwtConfig::new(key);
            if let Ok(iss) = std::env::var("AGNOSAI_JWT_ISSUER") {
                cfg = cfg.with_issuer(iss);
            }
            if let Ok(aud) = std::env::var("AGNOSAI_JWT_AUDIENCE") {
                cfg = cfg.with_audience(aud);
            }
            cfg
        });

    if enabled && secret.is_empty() && jwt.is_none() {
        tracing::warn!(
            "Auth enabled but no secret or JWT key configured — all requests will be rejected"
        );
    }

    if let Some(ref jwt_cfg) = jwt {
        AuthConfig::with_jwt(jwt_cfg.clone())
    } else if !secret.is_empty() {
        AuthConfig::with_secret(secret)
    } else if enabled {
        // Enabled but no credentials — will reject everything.
        let mut cfg = AuthConfig::default();
        cfg.enabled = true;
        cfg
    } else {
        AuthConfig::default()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialise tracing. When the `otel` feature is enabled and
    // OTEL_EXPORTER_OTLP_ENDPOINT is set, spans are exported via OTLP.
    #[cfg(feature = "otel")]
    let _tracing_guard = {
        let endpoint = agnosai::telemetry::otlp_endpoint_from_env();
        agnosai::telemetry::init_tracing(endpoint.as_deref())?
    };

    #[cfg(not(feature = "otel"))]
    {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env().add_directive("agnosai=info".parse()?))
            .json()
            .init();
    }

    tracing::info!("AgnosAI server starting");

    // Initialise shared state.
    let events = EventBus::new();
    let hoosh_url = std::env::var("HOOSH_URL").unwrap_or_else(|_| "http://localhost:8088".into());
    tracing::info!(hoosh_url = %hoosh_url, "LLM client configured (lazy — init on first use)");

    let orchestrator = Orchestrator::new(Default::default())
        .await?
        .with_llm_url(&hoosh_url)
        .with_events(events.clone());
    let tools = Arc::new(ToolRegistry::new());
    tools.register(Arc::new(EchoTool));
    tools.register(Arc::new(JsonTransformTool));
    tools.register(Arc::new(LoadTestingTool));
    tools.register(Arc::new(SecurityAuditTool));
    tracing::info!(tools = tools.count(), "tools registered");

    let auth = load_auth_config();
    if auth.enabled {
        tracing::info!(jwt = auth.jwt.is_some(), "authentication enabled");
    } else {
        tracing::warn!("authentication disabled — set AGNOSAI_AUTH_ENABLED=true for production");
    }

    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .pool_max_idle_per_host(10)
        .build()?;

    let audit = orchestrator.audit().clone();
    let state: SharedState = Arc::new(AppState {
        orchestrator,
        tools,
        auth,
        events,
        http_client,
        audit,
    });

    let app = router(state);

    let port: u16 = std::env::var("PORT")
        .or_else(|_| std::env::var("AGNOSAI_PORT"))
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("server shut down gracefully");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("received SIGINT, shutting down"),
        _ = terminate => tracing::info!("received SIGTERM, shutting down"),
    }
}
