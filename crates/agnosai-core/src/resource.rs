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
