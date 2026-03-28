//! Topology-aware fleet scheduling — NVLink/XGMI-aware placement.
//!
//! Extends placement scoring to prefer nodes where GPU devices are
//! interconnected via high-bandwidth links, enabling efficient model
//! parallelism for large models.

use crate::core::resource::{AcceleratorType, HardwareInventory};
use serde::{Deserialize, Serialize};

/// Interconnect type between devices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum InterconnectType {
    /// No direct link (PCIe only).
    Pcie,
    /// NVIDIA NVLink.
    NvLink,
    /// AMD XGMI / Infinity Fabric.
    Xgmi,
    /// Intel CXL.
    Cxl,
}

/// Topology information for a node's devices.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NodeTopology {
    /// Device-to-device interconnect bandwidth in GB/s.
    /// Key: `(device_index_a, device_index_b)`.
    pub links: Vec<DeviceLink>,
}

/// A link between two devices.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DeviceLink {
    pub device_a: usize,
    pub device_b: usize,
    pub interconnect: InterconnectType,
    /// Bandwidth in GB/s.
    pub bandwidth_gbps: f64,
}

/// Score a node's topology for multi-GPU workloads.
///
/// Returns a score from 0.0 (no inter-device links) to 1.0 (all devices
/// connected via high-bandwidth links).
#[must_use]
pub fn topology_score(topology: &NodeTopology, inventory: &HardwareInventory) -> f64 {
    let gpu_count = inventory
        .devices
        .iter()
        .filter(|d| d.accelerator != AcceleratorType::Cpu)
        .count();

    if gpu_count <= 1 {
        return 1.0; // Single GPU — topology is irrelevant.
    }

    if topology.links.is_empty() {
        return 0.0; // No topology data — assume PCIe only.
    }

    // Score based on fraction of device pairs with high-bandwidth links.
    let total_pairs = gpu_count * (gpu_count - 1) / 2;
    if total_pairs == 0 {
        return 1.0;
    }

    let high_bw_links = topology
        .links
        .iter()
        .filter(|l| l.bandwidth_gbps >= 50.0) // NVLink3+ threshold
        .count();

    high_bw_links as f64 / total_pairs as f64
}

/// Check if a node's topology supports efficient tensor parallelism
/// across the requested number of devices.
#[must_use]
pub fn supports_tensor_parallel(topology: &NodeTopology, device_count: usize) -> bool {
    if device_count <= 1 {
        return true;
    }
    // Need at least (device_count - 1) high-bandwidth links for a ring topology.
    let high_bw = topology
        .links
        .iter()
        .filter(|l| l.bandwidth_gbps >= 50.0)
        .count();
    high_bw >= device_count - 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::resource::ComputeDevice;

    fn gpu_inventory(count: usize) -> HardwareInventory {
        HardwareInventory {
            cpu_cores: 16,
            memory_total_mb: 64_000,
            devices: (0..count)
                .map(|i| ComputeDevice {
                    index: i,
                    name: format!("GPU-{i}"),
                    accelerator: AcceleratorType::Cuda,
                    memory_total_mb: 80_000,
                    memory_available_mb: 80_000,
                })
                .collect(),
        }
    }

    #[test]
    fn single_gpu_scores_perfectly() {
        let topo = NodeTopology::default();
        assert_eq!(topology_score(&topo, &gpu_inventory(1)), 1.0);
    }

    #[test]
    fn no_links_scores_zero() {
        let topo = NodeTopology::default();
        assert_eq!(topology_score(&topo, &gpu_inventory(4)), 0.0);
    }

    #[test]
    fn full_nvlink_scores_one() {
        let topo = NodeTopology {
            links: vec![
                DeviceLink {
                    device_a: 0,
                    device_b: 1,
                    interconnect: InterconnectType::NvLink,
                    bandwidth_gbps: 600.0,
                },
                DeviceLink {
                    device_a: 0,
                    device_b: 2,
                    interconnect: InterconnectType::NvLink,
                    bandwidth_gbps: 600.0,
                },
                DeviceLink {
                    device_a: 0,
                    device_b: 3,
                    interconnect: InterconnectType::NvLink,
                    bandwidth_gbps: 600.0,
                },
                DeviceLink {
                    device_a: 1,
                    device_b: 2,
                    interconnect: InterconnectType::NvLink,
                    bandwidth_gbps: 600.0,
                },
                DeviceLink {
                    device_a: 1,
                    device_b: 3,
                    interconnect: InterconnectType::NvLink,
                    bandwidth_gbps: 600.0,
                },
                DeviceLink {
                    device_a: 2,
                    device_b: 3,
                    interconnect: InterconnectType::NvLink,
                    bandwidth_gbps: 600.0,
                },
            ],
        };
        assert_eq!(topology_score(&topo, &gpu_inventory(4)), 1.0);
    }

    #[test]
    fn partial_links_partial_score() {
        let topo = NodeTopology {
            links: vec![DeviceLink {
                device_a: 0,
                device_b: 1,
                interconnect: InterconnectType::NvLink,
                bandwidth_gbps: 600.0,
            }],
        };
        let score = topology_score(&topo, &gpu_inventory(4));
        assert!(score > 0.0 && score < 1.0);
    }

    #[test]
    fn tensor_parallel_single_device() {
        assert!(supports_tensor_parallel(&NodeTopology::default(), 1));
    }

    #[test]
    fn tensor_parallel_needs_links() {
        let topo = NodeTopology {
            links: vec![
                DeviceLink {
                    device_a: 0,
                    device_b: 1,
                    interconnect: InterconnectType::NvLink,
                    bandwidth_gbps: 600.0,
                },
                DeviceLink {
                    device_a: 1,
                    device_b: 2,
                    interconnect: InterconnectType::NvLink,
                    bandwidth_gbps: 600.0,
                },
                DeviceLink {
                    device_a: 2,
                    device_b: 3,
                    interconnect: InterconnectType::NvLink,
                    bandwidth_gbps: 600.0,
                },
            ],
        };
        assert!(supports_tensor_parallel(&topo, 4));
        assert!(!supports_tensor_parallel(&NodeTopology::default(), 4));
    }
}
