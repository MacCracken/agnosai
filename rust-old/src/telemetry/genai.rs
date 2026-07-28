//! OpenTelemetry GenAI semantic convention span helpers.
//!
//! Emits spans with standardized attributes per the
//! [OpenTelemetry GenAI Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/gen-ai/)
//! (v1.37+).
//!
//! These helpers create `tracing` spans with the correct attribute names
//! so that OTLP-compatible backends (Datadog, Grafana, Arize) can
//! categorize and display agent/inference operations.

/// Standard attribute keys per OTel GenAI semantic conventions.
pub mod attrs {
    /// The name of the GenAI operation (e.g. "invoke_agent", "execute_tool").
    pub const OPERATION_NAME: &str = "gen_ai.operation.name";
    /// The GenAI system/provider (e.g. "openai", "anthropic", "ollama").
    pub const SYSTEM: &str = "gen_ai.system";
    /// The model requested for inference.
    pub const REQUEST_MODEL: &str = "gen_ai.request.model";
    /// The model that actually responded (may differ from requested).
    pub const RESPONSE_MODEL: &str = "gen_ai.response.model";
    /// Number of input/prompt tokens.
    pub const USAGE_INPUT_TOKENS: &str = "gen_ai.usage.input_tokens";
    /// Number of output/completion tokens.
    pub const USAGE_OUTPUT_TOKENS: &str = "gen_ai.usage.output_tokens";
    /// The reason the model stopped generating (e.g. "stop", "length").
    pub const RESPONSE_FINISH_REASON: &str = "gen_ai.response.finish_reason";
    /// Agent name.
    pub const AGENT_NAME: &str = "gen_ai.agent.name";
    /// Agent identifier.
    pub const AGENT_ID: &str = "gen_ai.agent.id";
    /// Tool name (AgnosAI-specific — not in OTel GenAI spec).
    pub const TOOL_NAME: &str = "agnosai.tool.name";
    /// Crew identifier.
    pub const CREW_ID: &str = "agnosai.crew.id";
    /// Crew name.
    pub const CREW_NAME: &str = "agnosai.crew.name";
    /// Task identifier.
    pub const TASK_ID: &str = "agnosai.task.id";
    /// Inference temperature.
    pub const REQUEST_TEMPERATURE: &str = "gen_ai.request.temperature";
    /// Maximum tokens requested.
    pub const REQUEST_MAX_TOKENS: &str = "gen_ai.request.max_tokens";
}

/// Create a tracing span for an agent inference call with GenAI attributes.
///
/// Call this macro at the start of an inference request. The span will be
/// active until dropped.
///
/// # Example
/// ```ignore
/// let _span = agnosai::telemetry::genai::inference_span(
///     "llama3:70b", "ollama", "agent-analyst", "task-123",
/// );
/// ```
#[must_use]
pub fn inference_span(model: &str, system: &str, agent_name: &str, task_id: &str) -> tracing::Span {
    tracing::info_span!(
        "gen_ai.invoke_agent",
        { attrs::OPERATION_NAME } = "invoke_agent",
        { attrs::SYSTEM } = system,
        { attrs::REQUEST_MODEL } = model,
        { attrs::AGENT_NAME } = agent_name,
        { attrs::TASK_ID } = task_id,
        { attrs::USAGE_INPUT_TOKENS } = tracing::field::Empty,
        { attrs::USAGE_OUTPUT_TOKENS } = tracing::field::Empty,
        { attrs::RESPONSE_MODEL } = tracing::field::Empty,
    )
}

/// Record token usage on an existing span.
pub fn record_usage(span: &tracing::Span, input_tokens: u64, output_tokens: u64, model: &str) {
    span.record(attrs::USAGE_INPUT_TOKENS, input_tokens);
    span.record(attrs::USAGE_OUTPUT_TOKENS, output_tokens);
    span.record(attrs::RESPONSE_MODEL, model);
}

/// Create a tracing span for a tool execution with GenAI attributes.
#[must_use]
pub fn tool_span(tool_name: &str, agent_name: &str, task_id: &str) -> tracing::Span {
    tracing::info_span!(
        "gen_ai.execute_tool",
        { attrs::OPERATION_NAME } = "execute_tool",
        { attrs::TOOL_NAME } = tool_name,
        { attrs::AGENT_NAME } = agent_name,
        { attrs::TASK_ID } = task_id,
    )
}

/// Create a tracing span for a crew execution.
#[must_use]
pub fn crew_span(crew_id: &str, crew_name: &str, task_count: usize) -> tracing::Span {
    tracing::info_span!(
        "agnosai.crew.run",
        { attrs::CREW_ID } = crew_id,
        { attrs::CREW_NAME } = crew_name,
        "agnosai.crew.task_count" = task_count,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attr_constants_follow_otel_naming() {
        assert!(attrs::OPERATION_NAME.starts_with("gen_ai."));
        assert!(attrs::SYSTEM.starts_with("gen_ai."));
        assert!(attrs::REQUEST_MODEL.starts_with("gen_ai."));
        assert!(attrs::USAGE_INPUT_TOKENS.starts_with("gen_ai."));
        assert!(attrs::USAGE_OUTPUT_TOKENS.starts_with("gen_ai."));
        assert!(attrs::AGENT_NAME.starts_with("gen_ai."));
    }

    #[test]
    fn inference_span_creates_span() {
        let span = inference_span("llama3:70b", "ollama", "agent-a", "task-1");
        assert!(span.is_disabled() || !span.is_disabled()); // Just verify it doesn't panic.
    }

    #[test]
    fn tool_span_creates_span() {
        let span = tool_span("echo", "agent-a", "task-1");
        let _ = span; // Verify creation doesn't panic.
    }

    #[test]
    fn crew_span_creates_span() {
        let span = crew_span("crew-123", "test-crew", 5);
        let _ = span;
    }

    #[test]
    fn record_usage_does_not_panic() {
        let span = inference_span("gpt-4", "openai", "agent-1", "task-1");
        record_usage(&span, 100, 50, "gpt-4-0613");
    }

    #[test]
    fn all_attr_constants_are_non_empty() {
        let all = [
            attrs::OPERATION_NAME,
            attrs::SYSTEM,
            attrs::REQUEST_MODEL,
            attrs::RESPONSE_MODEL,
            attrs::USAGE_INPUT_TOKENS,
            attrs::USAGE_OUTPUT_TOKENS,
            attrs::RESPONSE_FINISH_REASON,
            attrs::AGENT_NAME,
            attrs::AGENT_ID,
            attrs::TOOL_NAME,
            attrs::CREW_ID,
            attrs::CREW_NAME,
            attrs::TASK_ID,
            attrs::REQUEST_TEMPERATURE,
            attrs::REQUEST_MAX_TOKENS,
        ];
        for attr in all {
            assert!(!attr.is_empty(), "attribute constant should not be empty");
            assert!(
                attr.contains('.'),
                "attribute '{attr}' should follow dotted naming convention"
            );
        }
    }

    #[test]
    fn otel_standard_attrs_use_gen_ai_prefix() {
        // OTel GenAI spec requires gen_ai.* prefix for standard attributes.
        let otel_attrs = [
            attrs::OPERATION_NAME,
            attrs::SYSTEM,
            attrs::REQUEST_MODEL,
            attrs::RESPONSE_MODEL,
            attrs::USAGE_INPUT_TOKENS,
            attrs::USAGE_OUTPUT_TOKENS,
            attrs::RESPONSE_FINISH_REASON,
            attrs::AGENT_NAME,
            attrs::AGENT_ID,
            attrs::REQUEST_TEMPERATURE,
            attrs::REQUEST_MAX_TOKENS,
        ];
        for attr in otel_attrs {
            assert!(
                attr.starts_with("gen_ai."),
                "OTel attribute '{attr}' must start with gen_ai."
            );
        }
    }

    #[test]
    fn agnosai_attrs_use_agnosai_prefix() {
        let custom_attrs = [
            attrs::TOOL_NAME,
            attrs::CREW_ID,
            attrs::CREW_NAME,
            attrs::TASK_ID,
        ];
        for attr in custom_attrs {
            assert!(
                attr.starts_with("agnosai."),
                "custom attribute '{attr}' must start with agnosai."
            );
        }
    }
}
