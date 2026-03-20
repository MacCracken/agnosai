//! Task-complexity model routing.
//!
//! Automatically selects the appropriate model tier based on task profile:
//! - Fast: summarization, classification, simple tasks
//! - Capable: code generation, planning, reasoning (medium complexity)
//! - Premium: research, multi-step workflows, complex reasoning

use serde::{Deserialize, Serialize};

/// Model tier — maps to concrete models per provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelTier {
    /// Cheapest / fastest models (e.g. gpt-4o-mini, haiku, gemma).
    Fast,
    /// Balanced cost/quality (e.g. gpt-4o, sonnet, llama-3.1-70b).
    Capable,
    /// Highest quality (e.g. o3, opus, llama-3.1-405b).
    Premium,
}

/// What kind of task is being performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    Summarize,
    Classify,
    Code,
    Plan,
    Reason,
    Research,
    MultiStep,
}

/// How complex the task is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Complexity {
    Simple,
    Medium,
    Complex,
}

/// A description of a task for routing purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProfile {
    pub task_type: TaskType,
    pub complexity: Complexity,
}

/// Select the appropriate model tier for a given task profile.
///
/// The routing matrix:
///
/// | TaskType    | Simple  | Medium  | Complex |
/// |-------------|---------|---------|---------|
/// | Summarize   | Fast    | Fast    | Capable |
/// | Classify    | Fast    | Fast    | Capable |
/// | Code        | Capable | Capable | Premium |
/// | Plan        | Capable | Capable | Premium |
/// | Reason      | Capable | Capable | Premium |
/// | Research    | Capable | Premium | Premium |
/// | MultiStep   | Capable | Premium | Premium |
pub fn route(profile: &TaskProfile) -> ModelTier {
    use Complexity::*;
    use TaskType::*;

    match (&profile.task_type, &profile.complexity) {
        // Light tasks stay on fast tier unless complex.
        (Summarize | Classify, Simple | Medium) => ModelTier::Fast,
        (Summarize | Classify, Complex) => ModelTier::Capable,

        // Code / Plan / Reason need at least capable; complex → premium.
        (Code | Plan | Reason, Simple | Medium) => ModelTier::Capable,
        (Code | Plan | Reason, Complex) => ModelTier::Premium,

        // Research / MultiStep escalate earlier.
        (Research | MultiStep, Simple) => ModelTier::Capable,
        (Research | MultiStep, Medium | Complex) => ModelTier::Premium,
    }
}

/// Quantization recommendation for a model based on available hardware.
///
/// Uses `ai-hwaccel`'s `suggest_quantization()` to pick the best precision
/// level (FP32 / FP16 / BF16 / INT8 / INT4) that fits in available VRAM.
///
/// # Arguments
/// * `model_params` — approximate parameter count (e.g. 7_000_000_000 for a 7B model)
/// * `registry` — detected hardware from `ai_hwaccel::AcceleratorRegistry::detect()`
///
/// Returns a quantization level suitable for inference on the best available device.
#[cfg(feature = "hwaccel")]
pub fn suggest_quantization(
    model_params: u64,
    registry: &ai_hwaccel::AcceleratorRegistry,
) -> ai_hwaccel::QuantizationLevel {
    registry.suggest_quantization(model_params)
}

/// Estimate the memory required to load a model at a given quantization level.
///
/// Returns the estimated memory in bytes.
#[cfg(feature = "hwaccel")]
pub fn estimate_model_memory(
    model_params: u64,
    quant: &ai_hwaccel::QuantizationLevel,
) -> u64 {
    ai_hwaccel::AcceleratorRegistry::estimate_memory(model_params, quant)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarize_simple_is_fast() {
        let p = TaskProfile {
            task_type: TaskType::Summarize,
            complexity: Complexity::Simple,
        };
        assert_eq!(route(&p), ModelTier::Fast);
    }

    #[test]
    fn classify_medium_is_fast() {
        let p = TaskProfile {
            task_type: TaskType::Classify,
            complexity: Complexity::Medium,
        };
        assert_eq!(route(&p), ModelTier::Fast);
    }

    #[test]
    fn summarize_complex_is_capable() {
        let p = TaskProfile {
            task_type: TaskType::Summarize,
            complexity: Complexity::Complex,
        };
        assert_eq!(route(&p), ModelTier::Capable);
    }

    #[test]
    fn code_simple_is_capable() {
        let p = TaskProfile {
            task_type: TaskType::Code,
            complexity: Complexity::Simple,
        };
        assert_eq!(route(&p), ModelTier::Capable);
    }

    #[test]
    fn code_complex_is_premium() {
        let p = TaskProfile {
            task_type: TaskType::Code,
            complexity: Complexity::Complex,
        };
        assert_eq!(route(&p), ModelTier::Premium);
    }

    #[test]
    fn reason_medium_is_capable() {
        let p = TaskProfile {
            task_type: TaskType::Reason,
            complexity: Complexity::Medium,
        };
        assert_eq!(route(&p), ModelTier::Capable);
    }

    #[test]
    fn research_simple_is_capable() {
        let p = TaskProfile {
            task_type: TaskType::Research,
            complexity: Complexity::Simple,
        };
        assert_eq!(route(&p), ModelTier::Capable);
    }

    #[test]
    fn research_medium_is_premium() {
        let p = TaskProfile {
            task_type: TaskType::Research,
            complexity: Complexity::Medium,
        };
        assert_eq!(route(&p), ModelTier::Premium);
    }

    #[test]
    fn multistep_complex_is_premium() {
        let p = TaskProfile {
            task_type: TaskType::MultiStep,
            complexity: Complexity::Complex,
        };
        assert_eq!(route(&p), ModelTier::Premium);
    }

    #[test]
    fn plan_complex_is_premium() {
        let p = TaskProfile {
            task_type: TaskType::Plan,
            complexity: Complexity::Complex,
        };
        assert_eq!(route(&p), ModelTier::Premium);
    }

    #[cfg(feature = "hwaccel")]
    mod hwaccel_tests {
        use super::super::*;
        use ai_hwaccel::{AcceleratorProfile, AcceleratorRegistry, QuantizationLevel};

        #[test]
        fn suggest_quantization_small_model_high_vram() {
            // 7B model with 80GB GPU — should suggest FP16 or better.
            let registry = AcceleratorRegistry::from_profiles(vec![
                AcceleratorProfile::cpu(64 * 1024 * 1024 * 1024),
                AcceleratorProfile::cuda(0, 80 * 1024 * 1024 * 1024),
            ]);
            let quant = suggest_quantization(7_000_000_000, &registry);
            // With 80GB VRAM, a 7B model easily fits at FP16 or FP32.
            assert!(
                quant.bits_per_param() >= 16,
                "7B model with 80GB should get at least FP16, got {:?}",
                quant
            );
        }

        #[test]
        fn suggest_quantization_large_model_small_vram() {
            // 70B model with 24GB GPU — must quantize aggressively.
            let registry = AcceleratorRegistry::from_profiles(vec![
                AcceleratorProfile::cpu(32 * 1024 * 1024 * 1024),
                AcceleratorProfile::cuda(0, 24 * 1024 * 1024 * 1024),
            ]);
            let quant = suggest_quantization(70_000_000_000, &registry);
            // 70B at FP16 = ~140GB, way over 24GB. Must quantize.
            assert!(
                quant.bits_per_param() < 16,
                "70B model with 24GB should be quantized below FP16, got {:?}",
                quant
            );
        }

        #[test]
        fn estimate_model_memory_scales_with_quantization() {
            let fp32 = estimate_model_memory(7_000_000_000, &QuantizationLevel::None);
            let fp16 = estimate_model_memory(7_000_000_000, &QuantizationLevel::Float16);
            let int4 = estimate_model_memory(7_000_000_000, &QuantizationLevel::Int4);

            assert!(fp32 > fp16, "FP32 should use more memory than FP16");
            assert!(fp16 > int4, "FP16 should use more memory than INT4");
        }
    }
}
