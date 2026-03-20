use serde::{Deserialize, Serialize};

// Re-export ai-hwaccel's rich types when the feature is enabled, so callers
// can work with the full 19-variant AcceleratorType, quantization levels,
// sharding plans, and training methods without depending on ai-hwaccel directly.
#[cfg(feature = "hwaccel")]
pub use ai_hwaccel::{
    AcceleratorFamily as HwAccelFamily,
    AcceleratorRequirement as HwAccelRequirement,
    AcceleratorType as HwAccelType,
    QuantizationLevel,
    ShardingPlan, ShardingStrategy, ModelShard,
    TrainingMethod, TrainingTarget, MemoryEstimate,
};

/// Broad accelerator family for requirement matching.
///
/// Use this instead of matching on specific [`AcceleratorType`] variants when
/// you care about the *kind* of device (any GPU, any NPU) rather than the
/// specific vendor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcceleratorFamily {
    Cpu,
    Gpu,
    Npu,
    Tpu,
    AiAsic,
}

/// Hardware accelerator type.
///
/// These 6 broad categories cover the most common accelerator families.
/// When the `hwaccel` feature is enabled, use [`AcceleratorType::from_hwaccel`]
/// for lossless conversion from ai-hwaccel's 19-variant enum, and [`HwAccelType`]
/// when you need the full vendor-specific detail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcceleratorType {
    Cpu,
    Cuda,
    Rocm,
    Metal,
    Vulkan,
    Tpu,
}

impl AcceleratorType {
    /// Broad family this accelerator belongs to.
    pub fn family(self) -> AcceleratorFamily {
        match self {
            Self::Cpu => AcceleratorFamily::Cpu,
            Self::Cuda | Self::Rocm | Self::Metal | Self::Vulkan => AcceleratorFamily::Gpu,
            Self::Tpu => AcceleratorFamily::Tpu,
        }
    }

    /// Whether this is a GPU variant.
    pub fn is_gpu(self) -> bool {
        self.family() == AcceleratorFamily::Gpu
    }

    /// Whether this is a TPU variant.
    pub fn is_tpu(self) -> bool {
        self.family() == AcceleratorFamily::Tpu
    }
}

#[cfg(feature = "hwaccel")]
impl AcceleratorType {
    /// Map an `ai_hwaccel::AcceleratorType` to the simplified AgnosAI variant.
    ///
    /// Variants that don't have a direct mapping (NPUs, ASICs, etc.) map to `Cpu`
    /// as a safe fallback — the device is still usable but won't match GPU/TPU
    /// requirements in placement.
    pub fn from_hwaccel(hw: &ai_hwaccel::AcceleratorType) -> Self {
        use ai_hwaccel::AcceleratorType as HW;
        match hw {
            HW::Cpu => Self::Cpu,
            HW::CudaGpu { .. } => Self::Cuda,
            HW::RocmGpu { .. } => Self::Rocm,
            HW::MetalGpu => Self::Metal,
            HW::VulkanGpu { .. } => Self::Vulkan,
            HW::Tpu { .. } => Self::Tpu,
            _ => Self::Cpu,
        }
    }
}

/// A compute device with type, memory, and identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeDevice {
    pub index: usize,
    pub name: String,
    pub accelerator: AcceleratorType,
    pub memory_total_mb: u64,
    pub memory_available_mb: u64,
}

/// Hardware requirements for a task or agent workload.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HardwareRequirement {
    /// Acceptable accelerator types (empty = any, including CPU-only).
    #[serde(default)]
    pub accelerators: Vec<AcceleratorType>,
    /// Minimum device memory in MB (0 = no requirement).
    #[serde(default)]
    pub min_memory_mb: u64,
    /// Minimum number of devices needed (0 = no requirement).
    #[serde(default)]
    pub min_device_count: usize,
    /// Minimum CPU cores (0 = no requirement).
    #[serde(default)]
    pub min_cpu_cores: usize,
    /// Required accelerator family (None = match by specific type via `accelerators`).
    ///
    /// When set, any device in this family satisfies the requirement regardless
    /// of the `accelerators` list. This is useful for requirements like "any GPU"
    /// without enumerating Cuda, Rocm, Metal, Vulkan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_family: Option<AcceleratorFamily>,
}

/// Hardware inventory for a node.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HardwareInventory {
    pub cpu_cores: usize,
    pub memory_total_mb: u64,
    pub devices: Vec<ComputeDevice>,
}

impl HardwareInventory {
    /// Auto-detect hardware and populate inventory from system probes.
    ///
    /// Uses `ai-hwaccel`'s `AcceleratorRegistry::detect()` to probe for GPUs,
    /// TPUs, NPUs, and other accelerators via sysfs and CLI tools. CPU cores
    /// and system memory are also populated from the detected profiles.
    ///
    /// Only available when the `hwaccel` feature is enabled.
    #[cfg(feature = "hwaccel")]
    pub fn detect() -> Self {
        let registry = ai_hwaccel::AcceleratorRegistry::detect();
        Self::from_hwaccel(&registry)
    }

    /// Build a `HardwareInventory` from an `ai-hwaccel` `AcceleratorRegistry`.
    ///
    /// Maps each detected `AcceleratorProfile` into a [`ComputeDevice`], using
    /// [`AcceleratorType::from_hwaccel`] for the type mapping.
    #[cfg(feature = "hwaccel")]
    pub fn from_hwaccel(registry: &ai_hwaccel::AcceleratorRegistry) -> Self {
        let mut cpu_cores = 0_usize;
        let mut memory_total_mb = 0_u64;
        let mut devices = Vec::new();

        for (i, profile) in registry.all_profiles().iter().enumerate() {
            let accel = AcceleratorType::from_hwaccel(&profile.accelerator);
            let mem_mb = profile.memory_bytes / (1024 * 1024);
            let avail_mb = profile
                .memory_free_bytes
                .unwrap_or(profile.memory_bytes)
                / (1024 * 1024);

            if matches!(profile.accelerator, ai_hwaccel::AcceleratorType::Cpu) {
                // Use CPU memory as system memory total; estimate cores from
                // the profile (ai-hwaccel doesn't expose core count directly,
                // but we can use available_parallelism as a fallback).
                memory_total_mb = mem_mb;
                cpu_cores = std::thread::available_parallelism()
                    .map(|p| p.get())
                    .unwrap_or(1);
                continue;
            }

            devices.push(ComputeDevice {
                index: i,
                name: format!("{:?}", profile.accelerator),
                accelerator: accel,
                memory_total_mb: mem_mb,
                memory_available_mb: avail_mb,
            });
        }

        Self {
            cpu_cores,
            memory_total_mb,
            devices,
        }
    }

    /// Check if this inventory satisfies a hardware requirement.
    pub fn satisfies(&self, req: &HardwareRequirement) -> bool {
        // Check CPU cores
        if req.min_cpu_cores > 0 && self.cpu_cores < req.min_cpu_cores {
            return false;
        }

        // Family-based matching: "any GPU", "any TPU", etc.
        if let Some(family) = req.required_family {
            let matching_devices: Vec<_> = self
                .devices
                .iter()
                .filter(|d| d.accelerator.family() == family)
                .collect();
            if matching_devices.is_empty() {
                return false;
            }
            if req.min_device_count > 0 && matching_devices.len() < req.min_device_count {
                return false;
            }
            if req.min_memory_mb > 0
                && !matching_devices
                    .iter()
                    .any(|d| d.memory_total_mb >= req.min_memory_mb)
            {
                return false;
            }
            return true;
        }

        // Specific type matching (original behavior).
        if !req.accelerators.is_empty() {
            let matching_devices: Vec<_> = self
                .devices
                .iter()
                .filter(|d| req.accelerators.contains(&d.accelerator))
                .collect();
            // Check device count
            if req.min_device_count > 0 && matching_devices.len() < req.min_device_count {
                return false;
            }
            // Check memory per device
            if req.min_memory_mb > 0
                && !matching_devices
                    .iter()
                    .any(|d| d.memory_total_mb >= req.min_memory_mb)
            {
                return false;
            }
            // Must have at least one matching device
            if matching_devices.is_empty() {
                return false;
            }
        }
        true
    }

    /// List devices of a specific accelerator type.
    pub fn devices_of_type(&self, accel: AcceleratorType) -> Vec<&ComputeDevice> {
        self.devices
            .iter()
            .filter(|d| d.accelerator == accel)
            .collect()
    }

    /// Total device memory across all devices of a type.
    pub fn total_memory_mb(&self, accel: AcceleratorType) -> u64 {
        self.devices_of_type(accel)
            .iter()
            .map(|d| d.memory_total_mb)
            .sum()
    }
}

/// Training memory estimate for a model on specific hardware.
///
/// Wraps `ai-hwaccel`'s `estimate_training_memory()` to validate whether
/// an agent's workload fits in the available VRAM before scheduling.
#[cfg(feature = "hwaccel")]
#[derive(Debug, Clone)]
pub struct TrainingMemoryEstimate {
    /// Model weights memory in GB.
    pub model_gb: f64,
    /// Optimizer state memory in GB.
    pub optimizer_gb: f64,
    /// Activation memory in GB.
    pub activation_gb: f64,
    /// Total estimated memory in GB.
    pub total_gb: f64,
}

#[cfg(feature = "hwaccel")]
impl TrainingMemoryEstimate {
    /// Estimate training memory for a model with a given fine-tuning method.
    ///
    /// # Arguments
    /// * `model_params_millions` — parameter count in millions (e.g. 7000 for 7B)
    /// * `method` — training/fine-tuning method (FullFineTune, LoRA, QLoRA, etc.)
    /// * `target` — target accelerator type (Gpu, Tpu, Gaudi, Cpu)
    pub fn estimate(
        model_params_millions: u64,
        method: ai_hwaccel::TrainingMethod,
        target: ai_hwaccel::TrainingTarget,
    ) -> Self {
        let est = ai_hwaccel::estimate_training_memory(model_params_millions, method, target);
        Self {
            model_gb: est.model_gb,
            optimizer_gb: est.optimizer_gb,
            activation_gb: est.activation_gb,
            total_gb: est.total_gb,
        }
    }

    /// Check whether the estimated training memory fits in an inventory's
    /// total accelerator VRAM.
    pub fn fits_in(&self, inventory: &HardwareInventory) -> bool {
        let total_vram_gb = inventory
            .devices
            .iter()
            .map(|d| d.memory_total_mb as f64 / 1024.0)
            .sum::<f64>();
        self.total_gb <= total_vram_gb
    }

    /// Total memory in bytes.
    pub fn total_bytes(&self) -> u64 {
        (self.total_gb * 1024.0 * 1024.0 * 1024.0) as u64
    }
}

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
    fn accelerator_type_serde_round_trip_all_variants() {
        let variants = [
            (AcceleratorType::Cpu, "\"cpu\""),
            (AcceleratorType::Cuda, "\"cuda\""),
            (AcceleratorType::Rocm, "\"rocm\""),
            (AcceleratorType::Metal, "\"metal\""),
            (AcceleratorType::Vulkan, "\"vulkan\""),
            (AcceleratorType::Tpu, "\"tpu\""),
        ];
        for (variant, expected_json) in &variants {
            let json = serde_json::to_string(variant).unwrap();
            assert_eq!(&json, expected_json);
            let restored: AcceleratorType = serde_json::from_str(&json).unwrap();
            assert_eq!(*variant, restored);
        }
    }

    #[test]
    fn compute_device_serde_round_trip() {
        let device = ComputeDevice {
            index: 1,
            name: "NVIDIA H100".into(),
            accelerator: AcceleratorType::Cuda,
            memory_total_mb: 81920,
            memory_available_mb: 40960,
        };
        let json = serde_json::to_string(&device).unwrap();
        let restored: ComputeDevice = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.index, 1);
        assert_eq!(restored.name, "NVIDIA H100");
        assert_eq!(restored.accelerator, AcceleratorType::Cuda);
        assert_eq!(restored.memory_total_mb, 81920);
        assert_eq!(restored.memory_available_mb, 40960);
    }

    #[test]
    fn hardware_requirement_default_is_empty() {
        let req = HardwareRequirement::default();
        assert!(req.accelerators.is_empty());
        assert_eq!(req.min_memory_mb, 0);
        assert_eq!(req.min_device_count, 0);
        assert_eq!(req.min_cpu_cores, 0);
    }

    #[test]
    fn hardware_inventory_satisfies_cpu_only_requirement() {
        let inv = HardwareInventory {
            cpu_cores: 8,
            memory_total_mb: 32768,
            devices: vec![],
        };
        let req = HardwareRequirement {
            min_cpu_cores: 4,
            required_family: None,
            ..Default::default()
        };
        assert!(inv.satisfies(&req));

        let req_too_many = HardwareRequirement {
            min_cpu_cores: 16,
            required_family: None,
            ..Default::default()
        };
        assert!(!inv.satisfies(&req_too_many));
    }

    #[test]
    fn hardware_inventory_satisfies_cuda_gpu_requirement() {
        let inv = HardwareInventory {
            cpu_cores: 8,
            memory_total_mb: 32768,
            devices: vec![ComputeDevice {
                index: 0,
                name: "NVIDIA A100".into(),
                accelerator: AcceleratorType::Cuda,
                memory_total_mb: 81920,
                memory_available_mb: 40960,
            }],
        };
        let req = HardwareRequirement {
            accelerators: vec![AcceleratorType::Cuda],
            min_memory_mb: 40000,
            ..Default::default()
        };
        assert!(inv.satisfies(&req));
    }

    #[test]
    fn hardware_inventory_satisfies_tpu_passes_and_fails() {
        let inv_with_tpu = HardwareInventory {
            cpu_cores: 4,
            memory_total_mb: 16384,
            devices: vec![ComputeDevice {
                index: 0,
                name: "TPU v4".into(),
                accelerator: AcceleratorType::Tpu,
                memory_total_mb: 32768,
                memory_available_mb: 32768,
            }],
        };
        let req = HardwareRequirement {
            accelerators: vec![AcceleratorType::Tpu],
            ..Default::default()
        };
        assert!(inv_with_tpu.satisfies(&req));

        let inv_no_tpu = HardwareInventory {
            cpu_cores: 4,
            memory_total_mb: 16384,
            devices: vec![ComputeDevice {
                index: 0,
                name: "NVIDIA A100".into(),
                accelerator: AcceleratorType::Cuda,
                memory_total_mb: 81920,
                memory_available_mb: 40960,
            }],
        };
        assert!(!inv_no_tpu.satisfies(&req));
    }

    #[test]
    fn hardware_inventory_satisfies_min_memory_mb_check() {
        let inv = HardwareInventory {
            cpu_cores: 4,
            memory_total_mb: 16384,
            devices: vec![ComputeDevice {
                index: 0,
                name: "Small GPU".into(),
                accelerator: AcceleratorType::Cuda,
                memory_total_mb: 4096,
                memory_available_mb: 2048,
            }],
        };
        let req_ok = HardwareRequirement {
            accelerators: vec![AcceleratorType::Cuda],
            min_memory_mb: 4096,
            ..Default::default()
        };
        assert!(inv.satisfies(&req_ok));

        let req_too_much = HardwareRequirement {
            accelerators: vec![AcceleratorType::Cuda],
            min_memory_mb: 8192,
            ..Default::default()
        };
        assert!(!inv.satisfies(&req_too_much));
    }

    #[test]
    fn hardware_inventory_satisfies_min_device_count_check() {
        let inv = HardwareInventory {
            cpu_cores: 8,
            memory_total_mb: 65536,
            devices: vec![
                ComputeDevice {
                    index: 0,
                    name: "GPU 0".into(),
                    accelerator: AcceleratorType::Cuda,
                    memory_total_mb: 16384,
                    memory_available_mb: 16384,
                },
                ComputeDevice {
                    index: 1,
                    name: "GPU 1".into(),
                    accelerator: AcceleratorType::Cuda,
                    memory_total_mb: 16384,
                    memory_available_mb: 16384,
                },
            ],
        };
        let req_2 = HardwareRequirement {
            accelerators: vec![AcceleratorType::Cuda],
            min_device_count: 2,
            ..Default::default()
        };
        assert!(inv.satisfies(&req_2));

        let req_4 = HardwareRequirement {
            accelerators: vec![AcceleratorType::Cuda],
            min_device_count: 4,
            ..Default::default()
        };
        assert!(!inv.satisfies(&req_4));
    }

    #[test]
    fn hardware_inventory_satisfies_empty_accelerators_any_ok() {
        let inv = HardwareInventory {
            cpu_cores: 2,
            memory_total_mb: 8192,
            devices: vec![],
        };
        let req = HardwareRequirement::default();
        assert!(inv.satisfies(&req));
    }

    #[test]
    fn accelerator_type_family() {
        assert_eq!(AcceleratorType::Cpu.family(), AcceleratorFamily::Cpu);
        assert_eq!(AcceleratorType::Cuda.family(), AcceleratorFamily::Gpu);
        assert_eq!(AcceleratorType::Rocm.family(), AcceleratorFamily::Gpu);
        assert_eq!(AcceleratorType::Metal.family(), AcceleratorFamily::Gpu);
        assert_eq!(AcceleratorType::Vulkan.family(), AcceleratorFamily::Gpu);
        assert_eq!(AcceleratorType::Tpu.family(), AcceleratorFamily::Tpu);
    }

    #[test]
    fn accelerator_type_is_gpu() {
        assert!(AcceleratorType::Cuda.is_gpu());
        assert!(AcceleratorType::Rocm.is_gpu());
        assert!(!AcceleratorType::Cpu.is_gpu());
        assert!(!AcceleratorType::Tpu.is_gpu());
    }

    #[test]
    fn satisfies_with_required_family_any_gpu() {
        let inv = HardwareInventory {
            cpu_cores: 8,
            memory_total_mb: 32768,
            devices: vec![ComputeDevice {
                index: 0,
                name: "AMD MI300X".into(),
                accelerator: AcceleratorType::Rocm,
                memory_total_mb: 196608,
                memory_available_mb: 196608,
            }],
        };
        // Require "any GPU" — ROCm should match.
        let req = HardwareRequirement {
            required_family: Some(AcceleratorFamily::Gpu),
            ..Default::default()
        };
        assert!(inv.satisfies(&req));

        // Require TPU — ROCm should not match.
        let req_tpu = HardwareRequirement {
            required_family: Some(AcceleratorFamily::Tpu),
            ..Default::default()
        };
        assert!(!inv.satisfies(&req_tpu));
    }

    #[test]
    fn satisfies_family_with_memory_requirement() {
        let inv = HardwareInventory {
            cpu_cores: 8,
            memory_total_mb: 32768,
            devices: vec![ComputeDevice {
                index: 0,
                name: "Small GPU".into(),
                accelerator: AcceleratorType::Cuda,
                memory_total_mb: 8192,
                memory_available_mb: 8192,
            }],
        };
        let req = HardwareRequirement {
            required_family: Some(AcceleratorFamily::Gpu),
            min_memory_mb: 4096,
            ..Default::default()
        };
        assert!(inv.satisfies(&req));

        let req_too_much = HardwareRequirement {
            required_family: Some(AcceleratorFamily::Gpu),
            min_memory_mb: 16384,
            ..Default::default()
        };
        assert!(!inv.satisfies(&req_too_much));
    }

    #[test]
    fn hardware_inventory_devices_of_type_filters_correctly() {
        let inv = HardwareInventory {
            cpu_cores: 8,
            memory_total_mb: 65536,
            devices: vec![
                ComputeDevice {
                    index: 0,
                    name: "NVIDIA A100".into(),
                    accelerator: AcceleratorType::Cuda,
                    memory_total_mb: 81920,
                    memory_available_mb: 40960,
                },
                ComputeDevice {
                    index: 1,
                    name: "TPU v4".into(),
                    accelerator: AcceleratorType::Tpu,
                    memory_total_mb: 32768,
                    memory_available_mb: 32768,
                },
                ComputeDevice {
                    index: 2,
                    name: "NVIDIA A10".into(),
                    accelerator: AcceleratorType::Cuda,
                    memory_total_mb: 24576,
                    memory_available_mb: 24576,
                },
            ],
        };
        let cuda_devices = inv.devices_of_type(AcceleratorType::Cuda);
        assert_eq!(cuda_devices.len(), 2);
        let tpu_devices = inv.devices_of_type(AcceleratorType::Tpu);
        assert_eq!(tpu_devices.len(), 1);
        let metal_devices = inv.devices_of_type(AcceleratorType::Metal);
        assert_eq!(metal_devices.len(), 0);
    }

    #[test]
    fn hardware_inventory_total_memory_mb_sums_correctly() {
        let inv = HardwareInventory {
            cpu_cores: 8,
            memory_total_mb: 65536,
            devices: vec![
                ComputeDevice {
                    index: 0,
                    name: "GPU 0".into(),
                    accelerator: AcceleratorType::Cuda,
                    memory_total_mb: 16384,
                    memory_available_mb: 8192,
                },
                ComputeDevice {
                    index: 1,
                    name: "GPU 1".into(),
                    accelerator: AcceleratorType::Cuda,
                    memory_total_mb: 24576,
                    memory_available_mb: 12288,
                },
                ComputeDevice {
                    index: 2,
                    name: "TPU".into(),
                    accelerator: AcceleratorType::Tpu,
                    memory_total_mb: 32768,
                    memory_available_mb: 32768,
                },
            ],
        };
        assert_eq!(inv.total_memory_mb(AcceleratorType::Cuda), 16384 + 24576);
        assert_eq!(inv.total_memory_mb(AcceleratorType::Tpu), 32768);
        assert_eq!(inv.total_memory_mb(AcceleratorType::Rocm), 0);
    }

    #[cfg(feature = "hwaccel")]
    #[test]
    fn hwaccel_accelerator_type_mapping() {
        use ai_hwaccel::AcceleratorType as HW;

        assert_eq!(AcceleratorType::from_hwaccel(&HW::Cpu), AcceleratorType::Cpu);
        assert_eq!(
            AcceleratorType::from_hwaccel(&HW::CudaGpu { device_id: 0 }),
            AcceleratorType::Cuda
        );
        assert_eq!(
            AcceleratorType::from_hwaccel(&HW::RocmGpu { device_id: 0 }),
            AcceleratorType::Rocm
        );
        assert_eq!(
            AcceleratorType::from_hwaccel(&HW::MetalGpu),
            AcceleratorType::Metal
        );
        assert_eq!(
            AcceleratorType::from_hwaccel(&HW::VulkanGpu {
                device_id: 0,
                device_name: "test".into(),
            }),
            AcceleratorType::Vulkan
        );
        assert_eq!(
            AcceleratorType::from_hwaccel(&HW::Tpu {
                device_id: 0,
                chip_count: 1,
                version: ai_hwaccel::TpuVersion::V4,
            }),
            AcceleratorType::Tpu
        );
        // NPUs and ASICs fall back to Cpu
        assert_eq!(
            AcceleratorType::from_hwaccel(&HW::IntelNpu),
            AcceleratorType::Cpu
        );
        assert_eq!(
            AcceleratorType::from_hwaccel(&HW::GroqLpu { device_id: 0 }),
            AcceleratorType::Cpu
        );
    }

    #[cfg(feature = "hwaccel")]
    #[test]
    fn hwaccel_inventory_from_registry() {
        use ai_hwaccel::{AcceleratorProfile, AcceleratorRegistry};

        let registry = AcceleratorRegistry::from_profiles(vec![
            AcceleratorProfile::cpu(64 * 1024 * 1024 * 1024),
            AcceleratorProfile::cuda(0, 80 * 1024 * 1024 * 1024),
            AcceleratorProfile::cuda(1, 80 * 1024 * 1024 * 1024),
        ]);

        let inv = HardwareInventory::from_hwaccel(&registry);

        // CPU memory mapped to system memory
        assert_eq!(inv.memory_total_mb, 64 * 1024);
        assert!(inv.cpu_cores > 0);

        // Two CUDA devices
        assert_eq!(inv.devices.len(), 2);
        assert_eq!(inv.devices[0].accelerator, AcceleratorType::Cuda);
        assert_eq!(inv.devices[1].accelerator, AcceleratorType::Cuda);
        assert_eq!(inv.devices[0].memory_total_mb, 80 * 1024);
        assert_eq!(inv.devices[1].memory_total_mb, 80 * 1024);
    }

    #[cfg(feature = "hwaccel")]
    #[test]
    fn hwaccel_inventory_satisfies_after_detect() {
        use ai_hwaccel::{AcceleratorProfile, AcceleratorRegistry};

        let registry = AcceleratorRegistry::from_profiles(vec![
            AcceleratorProfile::cpu(32 * 1024 * 1024 * 1024),
            AcceleratorProfile::cuda(0, 24 * 1024 * 1024 * 1024),
        ]);
        let inv = HardwareInventory::from_hwaccel(&registry);

        let req = HardwareRequirement {
            accelerators: vec![AcceleratorType::Cuda],
            min_memory_mb: 20000,
            min_device_count: 1,
            min_cpu_cores: 0,
            required_family: None,
        };
        assert!(inv.satisfies(&req));

        let req_too_much = HardwareRequirement {
            accelerators: vec![AcceleratorType::Cuda],
            min_memory_mb: 50000,
            min_device_count: 1,
            min_cpu_cores: 0,
            required_family: None,
        };
        assert!(!inv.satisfies(&req_too_much));
    }

    #[cfg(feature = "hwaccel")]
    #[test]
    fn hwaccel_empty_registry_gives_cpu_only() {
        use ai_hwaccel::{AcceleratorProfile, AcceleratorRegistry};

        let registry = AcceleratorRegistry::from_profiles(vec![
            AcceleratorProfile::cpu(16 * 1024 * 1024 * 1024),
        ]);
        let inv = HardwareInventory::from_hwaccel(&registry);

        assert!(inv.devices.is_empty());
        assert_eq!(inv.memory_total_mb, 16 * 1024);
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

    #[cfg(feature = "hwaccel")]
    mod training_tests {
        use super::super::*;

        #[test]
        fn estimate_full_finetune_7b() {
            let est = TrainingMemoryEstimate::estimate(
                7000,
                ai_hwaccel::TrainingMethod::FullFineTune,
                ai_hwaccel::TrainingTarget::Gpu,
            );
            // Full fine-tune of 7B needs model + optimizer + activations.
            assert!(est.total_gb > 0.0, "total should be positive");
            assert!(est.model_gb > 0.0, "model memory should be positive");
            assert!(est.optimizer_gb > 0.0, "optimizer memory should be positive");
            assert!(
                est.total_gb >= est.model_gb,
                "total should be >= model alone"
            );
        }

        #[test]
        fn estimate_lora_uses_less_memory() {
            let full = TrainingMemoryEstimate::estimate(
                7000,
                ai_hwaccel::TrainingMethod::FullFineTune,
                ai_hwaccel::TrainingTarget::Gpu,
            );
            let lora = TrainingMemoryEstimate::estimate(
                7000,
                ai_hwaccel::TrainingMethod::LoRA,
                ai_hwaccel::TrainingTarget::Gpu,
            );
            assert!(
                lora.total_gb < full.total_gb,
                "LoRA ({:.1} GB) should use less memory than full fine-tune ({:.1} GB)",
                lora.total_gb,
                full.total_gb
            );
        }

        #[test]
        fn estimate_qlora_uses_less_than_lora() {
            let lora = TrainingMemoryEstimate::estimate(
                7000,
                ai_hwaccel::TrainingMethod::LoRA,
                ai_hwaccel::TrainingTarget::Gpu,
            );
            let qlora = TrainingMemoryEstimate::estimate(
                7000,
                ai_hwaccel::TrainingMethod::QLoRA { bits: 4 },
                ai_hwaccel::TrainingTarget::Gpu,
            );
            assert!(
                qlora.total_gb < lora.total_gb,
                "QLoRA ({:.1} GB) should use less memory than LoRA ({:.1} GB)",
                qlora.total_gb,
                lora.total_gb
            );
        }

        #[test]
        fn fits_in_inventory() {
            let est = TrainingMemoryEstimate::estimate(
                7000,
                ai_hwaccel::TrainingMethod::LoRA,
                ai_hwaccel::TrainingTarget::Gpu,
            );

            let big_inv = HardwareInventory {
                cpu_cores: 8,
                memory_total_mb: 65536,
                devices: vec![ComputeDevice {
                    index: 0,
                    name: "A100 80GB".into(),
                    accelerator: AcceleratorType::Cuda,
                    memory_total_mb: 81920,
                    memory_available_mb: 81920,
                }],
            };
            assert!(
                est.fits_in(&big_inv),
                "7B LoRA ({:.1} GB) should fit in 80GB GPU",
                est.total_gb
            );

            let small_inv = HardwareInventory {
                cpu_cores: 4,
                memory_total_mb: 16384,
                devices: vec![ComputeDevice {
                    index: 0,
                    name: "Small GPU".into(),
                    accelerator: AcceleratorType::Cuda,
                    memory_total_mb: 4096,
                    memory_available_mb: 4096,
                }],
            };
            // 7B LoRA needs more than 4GB.
            assert!(
                !est.fits_in(&small_inv),
                "7B LoRA ({:.1} GB) should not fit in 4GB GPU",
                est.total_gb
            );
        }

        #[test]
        fn total_bytes_consistent() {
            let est = TrainingMemoryEstimate::estimate(
                7000,
                ai_hwaccel::TrainingMethod::FullFineTune,
                ai_hwaccel::TrainingTarget::Gpu,
            );
            let expected = (est.total_gb * 1024.0 * 1024.0 * 1024.0) as u64;
            assert_eq!(est.total_bytes(), expected);
        }
    }
}
