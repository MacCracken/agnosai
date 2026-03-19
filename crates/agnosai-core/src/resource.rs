use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceBudget {
    pub max_tokens: Option<u64>,
    pub max_cost_usd: Option<f64>,
    pub max_duration_secs: Option<u64>,
    pub max_concurrent_tasks: Option<usize>,
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self {
            max_tokens: None,
            max_cost_usd: None,
            max_duration_secs: None,
            max_concurrent_tasks: Some(10),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuDevice {
    pub index: usize,
    pub name: String,
    pub vram_total_mb: u64,
    pub vram_available_mb: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_budget_default_values() {
        let budget = ResourceBudget::default();
        assert!(budget.max_tokens.is_none());
        assert!(budget.max_cost_usd.is_none());
        assert!(budget.max_duration_secs.is_none());
        assert_eq!(budget.max_concurrent_tasks, Some(10));
    }

    #[test]
    fn resource_budget_serde_round_trip() {
        let budget = ResourceBudget {
            max_tokens: Some(50000),
            max_cost_usd: Some(1.5),
            max_duration_secs: Some(300),
            max_concurrent_tasks: Some(4),
        };
        let json = serde_json::to_string(&budget).unwrap();
        let restored: ResourceBudget = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.max_tokens, Some(50000));
        assert_eq!(restored.max_cost_usd, Some(1.5));
        assert_eq!(restored.max_duration_secs, Some(300));
        assert_eq!(restored.max_concurrent_tasks, Some(4));
    }

    #[test]
    fn resource_budget_serde_with_none_fields() {
        let budget = ResourceBudget::default();
        let json = serde_json::to_string(&budget).unwrap();
        let restored: ResourceBudget = serde_json::from_str(&json).unwrap();
        assert!(restored.max_tokens.is_none());
        assert!(restored.max_cost_usd.is_none());
        assert!(restored.max_duration_secs.is_none());
        assert_eq!(restored.max_concurrent_tasks, Some(10));
    }

    #[test]
    fn gpu_device_serde_round_trip() {
        let device = GpuDevice {
            index: 0,
            name: "NVIDIA A100".into(),
            vram_total_mb: 81920,
            vram_available_mb: 40960,
        };
        let json = serde_json::to_string(&device).unwrap();
        let restored: GpuDevice = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.index, 0);
        assert_eq!(restored.name, "NVIDIA A100");
        assert_eq!(restored.vram_total_mb, 81920);
        assert_eq!(restored.vram_available_mb, 40960);
    }
}
