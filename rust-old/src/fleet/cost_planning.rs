//! Cost-aware crew planning and model selection.
//!
//! Provides GPU pricing tables, crew cost estimation, and budget-aware model
//! selection so callers can plan crew executions within a dollar budget.

use std::fmt;

/// Per-hour GPU pricing for common accelerator types.
///
/// Prices are in USD per GPU-hour. These represent typical cloud spot/on-demand
/// rates and can be overridden by constructing with custom values.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct GpuPricing {
    /// NVIDIA A100 (80 GB) — USD per hour.
    pub a100: f64,
    /// NVIDIA H100 (80 GB) — USD per hour.
    pub h100: f64,
    /// NVIDIA RTX 4090 (24 GB) — USD per hour.
    pub rtx4090: f64,
    /// NVIDIA T4 (16 GB) — USD per hour.
    pub t4: f64,
    /// NVIDIA L4 (24 GB) — USD per hour.
    pub l4: f64,
}

impl Default for GpuPricing {
    /// Returns pricing as of early 2026 cloud spot rates.
    fn default() -> Self {
        Self {
            a100: 3.00,
            h100: 4.50,
            rtx4090: 1.20,
            t4: 0.35,
            l4: 0.80,
        }
    }
}

impl fmt::Display for GpuPricing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "A100=${:.2}/h H100=${:.2}/h RTX4090=${:.2}/h T4=${:.2}/h L4=${:.2}/h",
            self.a100, self.h100, self.rtx4090, self.t4, self.l4
        )
    }
}

/// Average tokens per second for each model tier, used to convert token counts
/// into GPU-hours.
///
/// Returns `(model_label, tokens_per_second, gpu_hourly_rate)`.
#[must_use]
fn model_throughput(model: &str, pricing: &GpuPricing) -> Option<(&'static str, f64, f64)> {
    // Approximate tok/s and matching GPU for common model families.
    let lower = model.to_ascii_lowercase();
    if lower.contains("gpt-4") || lower.contains("claude-3-opus") {
        Some(("large", 40.0, pricing.a100))
    } else if lower.contains("gpt-3.5") || lower.contains("claude-3-sonnet") {
        Some(("medium", 80.0, pricing.l4))
    } else if lower.contains("llama-70b") || lower.contains("mixtral") {
        Some(("open-large", 50.0, pricing.a100))
    } else if lower.contains("llama-7b")
        || lower.contains("llama-8b")
        || lower.contains("mistral-7b")
    {
        Some(("open-small", 120.0, pricing.t4))
    } else {
        // Unknown model — assume medium tier on L4.
        Some(("unknown", 80.0, pricing.l4))
    }
}

/// Estimate the total USD cost of running a crew.
///
/// # Arguments
///
/// * `tasks` — number of tasks in the crew
/// * `avg_tokens_per_task` — average tokens (input + output) per task
/// * `model` — model identifier string (e.g. `"gpt-4"`, `"llama-7b"`)
/// * `pricing` — GPU hourly pricing table
///
/// # Returns
///
/// Estimated cost in USD. Returns `0.0` if the model is unrecognized.
#[must_use]
#[tracing::instrument(skip(pricing), fields(tasks, avg_tokens_per_task, model))]
pub fn estimate_crew_cost(
    tasks: usize,
    avg_tokens_per_task: u64,
    model: &str,
    pricing: &GpuPricing,
) -> f64 {
    let Some((_tier, tps, hourly)) = model_throughput(model, pricing) else {
        return 0.0;
    };

    let total_tokens = tasks as f64 * avg_tokens_per_task as f64;
    let seconds = total_tokens / tps;
    let hours = seconds / 3600.0;

    tracing::debug!(
        total_tokens,
        gpu_hours = hours,
        hourly_rate = hourly,
        "crew cost estimate"
    );

    hours * hourly
}

/// Known model entries for budget selection: `(name, tier_label)`.
const BUDGET_MODELS: &[&str] = &[
    "llama-7b",
    "mistral-7b",
    "gpt-3.5-turbo",
    "llama-70b",
    "mixtral-8x7b",
    "claude-3-sonnet",
    "gpt-4",
    "claude-3-opus",
];

/// Select the cheapest model that fits within a USD budget for the given workload.
///
/// Iterates known models from cheapest to most expensive, returning the first
/// whose estimated cost is within `budget_usd`.
///
/// # Returns
///
/// `Some(model_name)` if a model fits the budget, `None` otherwise.
#[must_use]
#[tracing::instrument(skip(pricing))]
pub fn select_cheapest_model(
    budget_usd: f64,
    tasks: usize,
    avg_tokens_per_task: u64,
    pricing: &GpuPricing,
) -> Option<&'static str> {
    // Build (model, cost) pairs and sort by cost ascending.
    let mut candidates: Vec<(&str, f64)> = BUDGET_MODELS
        .iter()
        .map(|&m| {
            (
                m,
                estimate_crew_cost(tasks, avg_tokens_per_task, m, pricing),
            )
        })
        .collect();
    candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    for (model, cost) in &candidates {
        if *cost <= budget_usd {
            tracing::info!(
                model,
                cost,
                budget_usd,
                "selected cheapest model within budget"
            );
            return Some(model);
        }
    }

    tracing::warn!(budget_usd, "no model fits within budget");
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_pricing_values() {
        let p = GpuPricing::default();
        assert!((p.a100 - 3.00).abs() < f64::EPSILON);
        assert!((p.h100 - 4.50).abs() < f64::EPSILON);
        assert!((p.rtx4090 - 1.20).abs() < f64::EPSILON);
        assert!((p.t4 - 0.35).abs() < f64::EPSILON);
        assert!((p.l4 - 0.80).abs() < f64::EPSILON);
    }

    #[test]
    fn estimate_crew_cost_gpt4() {
        let pricing = GpuPricing::default();
        let cost = estimate_crew_cost(10, 1000, "gpt-4", &pricing);
        // 10 * 1000 = 10_000 tokens at 40 tok/s = 250s = 0.0694h * $3.00
        let expected = (10_000.0 / 40.0 / 3600.0) * 3.00;
        assert!(
            (cost - expected).abs() < 1e-6,
            "expected {expected}, got {cost}"
        );
    }

    #[test]
    fn estimate_crew_cost_small_model() {
        let pricing = GpuPricing::default();
        let cost = estimate_crew_cost(10, 1000, "llama-7b", &pricing);
        // 10_000 tokens at 120 tok/s = 83.33s = 0.02315h * $0.35
        let expected = (10_000.0 / 120.0 / 3600.0) * 0.35;
        assert!(
            (cost - expected).abs() < 1e-6,
            "expected {expected}, got {cost}"
        );
    }

    #[test]
    fn estimate_zero_tasks() {
        let pricing = GpuPricing::default();
        let cost = estimate_crew_cost(0, 1000, "gpt-4", &pricing);
        assert!((cost - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn estimate_zero_tokens() {
        let pricing = GpuPricing::default();
        let cost = estimate_crew_cost(10, 0, "gpt-4", &pricing);
        assert!((cost - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn select_cheapest_model_within_budget() {
        let pricing = GpuPricing::default();
        // Very tight budget — should pick cheapest (llama-7b or similar).
        let model = select_cheapest_model(0.01, 5, 500, &pricing);
        assert!(model.is_some());
        let name = model.unwrap();
        // The cheapest model should be one of the small open models.
        assert!(
            name == "llama-7b" || name == "mistral-7b",
            "expected cheap model, got {name}"
        );
    }

    #[test]
    fn select_cheapest_model_large_budget() {
        let pricing = GpuPricing::default();
        // Generous budget — should still pick cheapest.
        let model = select_cheapest_model(100.0, 5, 500, &pricing);
        assert!(model.is_some());
    }

    #[test]
    fn select_cheapest_model_zero_budget() {
        let pricing = GpuPricing::default();
        // Zero budget with actual work — nothing fits.
        let model = select_cheapest_model(0.0, 100, 10_000, &pricing);
        // With 0 budget and real work, no model should fit (costs > 0).
        // However, with very few tasks it might be ~0. Let's use big workload.
        assert!(
            model.is_none(),
            "expected None for zero budget with big workload"
        );
    }

    #[test]
    fn pricing_display() {
        let p = GpuPricing::default();
        let s = format!("{p}");
        assert!(s.contains("A100=$3.00/h"));
        assert!(s.contains("T4=$0.35/h"));
    }

    #[test]
    fn unknown_model_uses_medium_tier() {
        let pricing = GpuPricing::default();
        let cost = estimate_crew_cost(10, 1000, "some-custom-model", &pricing);
        // Should use "unknown" tier: 80 tok/s on L4 ($0.80/h)
        let expected = (10_000.0 / 80.0 / 3600.0) * 0.80;
        assert!(
            (cost - expected).abs() < 1e-6,
            "expected {expected}, got {cost}"
        );
    }
}
