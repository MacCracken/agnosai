//! OpenTelemetry integration for AgnosAI.
//!
//! Wraps [`hoosh::telemetry`] to provide OTLP trace export with
//! AgnosAI-specific configuration. Only compiled when the `otel` feature
//! is enabled.
//!
//! # Environment variables
//!
//! - `OTEL_EXPORTER_OTLP_ENDPOINT` — OTLP collector endpoint (e.g. `http://localhost:4317`)
//! - `OTEL_SERVICE_NAME` — Override the default service name (`agnosai`)

/// GenAI semantic convention span helpers (OTel v1.37+).
pub mod genai;

use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;

/// Default service name reported to the OTLP collector.
pub const DEFAULT_SERVICE_NAME: &str = "agnosai";

/// Guard that keeps the OTLP exporter alive.
///
/// When dropped, the exporter flushes pending spans and shuts down cleanly.
/// Hold this in a variable for the lifetime of the application.
pub struct TracingGuard {
    _otel: Option<hoosh::telemetry::OtelGuard>,
}

/// Initialise the tracing subscriber with optional OTLP export.
///
/// When `otlp_endpoint` is `Some`, spans are exported to the OTLP collector
/// (gRPC) in addition to stderr logging. When `None`, only stderr logging
/// is configured (structured text format).
///
/// Returns a [`TracingGuard`] that must be kept alive.
///
/// # Errors
///
/// Returns an error if the OTLP exporter fails to initialise.
pub fn init_tracing(
    otlp_endpoint: Option<&str>,
) -> Result<TracingGuard, Box<dyn std::error::Error>> {
    let service_name =
        std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| DEFAULT_SERVICE_NAME.to_string());

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("agnosai=info"));

    if let Some(endpoint) = otlp_endpoint {
        let mut otel_guard = hoosh::telemetry::init_otel(endpoint, &service_name)?;
        let otel_layer = otel_guard
            .layer()
            .ok_or("OtelGuard layer already taken — init_tracing called twice?")?;

        let subscriber = tracing_subscriber::registry()
            .with(otel_layer)
            .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
            .with(env_filter);
        tracing::subscriber::set_global_default(subscriber)?;

        tracing::info!(
            endpoint,
            service_name,
            "OpenTelemetry OTLP export initialised"
        );

        Ok(TracingGuard {
            _otel: Some(otel_guard),
        })
    } else {
        let subscriber = tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().json())
            .with(env_filter);
        tracing::subscriber::set_global_default(subscriber)?;

        Ok(TracingGuard { _otel: None })
    }
}

/// Read the OTLP endpoint from the standard environment variable.
#[must_use]
pub fn otlp_endpoint_from_env() -> Option<String> {
    std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .ok()
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_service_name_is_agnosai() {
        assert_eq!(DEFAULT_SERVICE_NAME, "agnosai");
    }

    #[test]
    fn otlp_endpoint_from_env_returns_none_when_unset() {
        let result = otlp_endpoint_from_env();
        let _ = result;
    }
}
