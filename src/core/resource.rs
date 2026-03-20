use serde::{Deserialize, Serialize};

/// Hardware accelerator type — compatibility shim over `ai-hwaccel`'s richer enum.
///
/// These 6 broad categories map from `ai-hwaccel`'s 19-variant `AcceleratorType`.
/// When the `hwaccel` feature is enabled, use [`AcceleratorType::from_hwaccel`] to
/// convert detected hardware into this simplified representation.
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
            // NPUs, ASICs, and other specialized hardware fall back to Cpu.
            // Phase 2 will replace this enum with ai-hwaccel's directly.
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
        // Check accelerator types
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
            ..Default::default()
        };
        assert!(inv.satisfies(&req));

        let req_too_many = HardwareRequirement {
            min_cpu_cores: 16,
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
        };
        assert!(inv.satisfies(&req));

        let req_too_much = HardwareRequirement {
            accelerators: vec![AcceleratorType::Cuda],
            min_memory_mb: 50000,
            min_device_count: 1,
            min_cpu_cores: 0,
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
}
