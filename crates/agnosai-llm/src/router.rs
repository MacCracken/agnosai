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
}
