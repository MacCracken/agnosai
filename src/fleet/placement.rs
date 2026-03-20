//! Scheduling policies: gpu-affinity, balanced, locality, cost, manual.

use crate::fleet::registry::{NodeId, NodeInfo, NodeStatus};

/// Scheduling policy for assigning crews/tasks to fleet nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementPolicy {
    /// Prefer nodes with GPU, then by available VRAM.
    GpuAffinity,
    /// Spread load evenly (round-robin among online nodes).
    Balanced,
    /// Prefer nodes with matching capabilities.
    Locality,
    /// Prefer nodes with least resources (minimize waste).
    Cost,
    /// Specific node ID required.
    Manual,
}

/// A request for node placement.
pub struct PlacementRequest {
    pub policy: PlacementPolicy,
    pub required_gpu: bool,
    pub min_gpu_vram_mb: u64,
    pub required_capabilities: Vec<String>,
    /// For Manual policy: the specific node to target.
    pub preferred_node: Option<NodeId>,
    /// Hardware requirements (new — takes precedence over legacy GPU fields).
    pub hardware: Option<crate::core::resource::HardwareRequirement>,
}

/// Result of a placement decision.
#[derive(Debug, Clone)]
pub struct PlacementResult {
    pub node_id: NodeId,
    pub score: f64,
}

/// Select the best node for a placement request from available nodes.
pub fn place(request: &PlacementRequest, nodes: &[&NodeInfo]) -> Option<PlacementResult> {
    rank_nodes(request, nodes).into_iter().next()
}

/// Score a single node for a request (0.0–1.0). Returns 0.0 if the node is
/// disqualified (offline, missing required GPU, insufficient VRAM).
fn score_node(request: &PlacementRequest, node: &NodeInfo, index: usize, count: usize) -> f64 {
    // Filter out non-Online nodes.
    if node.status != NodeStatus::Online {
        return 0.0;
    }

    // Hardware requirement check (takes precedence over legacy GPU fields).
    if let Some(hw_req) = &request.hardware {
        if !node.satisfies_hardware(hw_req) {
            return 0.0;
        }
    } else {
        // Legacy GPU checks as fallback when hardware is None.
        // Hard requirement: GPU.
        if request.required_gpu && !node.has_gpu() {
            return 0.0;
        }

        // Hard requirement: minimum VRAM.
        if request.min_gpu_vram_mb > 0 && node.gpu_vram_mb < request.min_gpu_vram_mb {
            return 0.0;
        }
    }

    match request.policy {
        PlacementPolicy::GpuAffinity => {
            if !node.has_gpu() {
                return 0.0;
            }
            // Score by VRAM; normalize to 0.0–1.0 using a reasonable ceiling.
            // More VRAM = higher score.
            let max_vram = 1_000_000.0_f64; // 1 TB ceiling for normalization
            (node.gpu_vram_mb as f64 / max_vram).min(1.0)
        }
        PlacementPolicy::Balanced => {
            // Spread load: score decreases with index position.
            if count <= 1 {
                1.0
            } else {
                1.0 - (index as f64 / count as f64)
            }
        }
        PlacementPolicy::Locality => {
            // Score by capability match fraction.
            if request.required_capabilities.is_empty() {
                return 1.0;
            }
            let matched = request
                .required_capabilities
                .iter()
                .filter(|cap| node.capabilities.contains(cap))
                .count();
            matched as f64 / request.required_capabilities.len() as f64
        }
        PlacementPolicy::Cost => {
            // Prefer nodes with least resources (minimize waste).
            // Lower resources = higher score.
            let resource_weight = node.gpu_count as f64 + node.gpu_vram_mb as f64;
            1.0 / (1.0 + resource_weight)
        }
        PlacementPolicy::Manual => {
            // Only the preferred node gets a score.
            match &request.preferred_node {
                Some(preferred) if preferred == &node.id => 1.0,
                _ => 0.0,
            }
        }
    }
}

/// Rank all online nodes, return sorted (best first).
pub fn rank_nodes(request: &PlacementRequest, nodes: &[&NodeInfo]) -> Vec<PlacementResult> {
    let count = nodes.len();
    let mut results: Vec<PlacementResult> = nodes
        .iter()
        .enumerate()
        .map(|(i, node)| PlacementResult {
            node_id: node.id.clone(),
            score: score_node(request, node, i, count),
        })
        .filter(|r| r.score > 0.0)
        .collect();

    // Sort descending by score.
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fleet::registry::NodeInfo;

    fn make_request(policy: PlacementPolicy) -> PlacementRequest {
        PlacementRequest {
            policy,
            required_gpu: false,
            min_gpu_vram_mb: 0,
            required_capabilities: Vec::new(),
            preferred_node: None,
            hardware: None,
        }
    }

    fn gpu_node(id: &str, vram: u64) -> NodeInfo {
        NodeInfo::new(id, 1, vram)
    }

    fn cpu_node(id: &str) -> NodeInfo {
        NodeInfo::new(id, 0, 0)
    }

    #[test]
    fn gpu_affinity_picks_most_vram() {
        let n1 = gpu_node("a", 8000);
        let n2 = gpu_node("b", 24000);
        let n3 = gpu_node("c", 16000);
        let nodes: Vec<&NodeInfo> = vec![&n1, &n2, &n3];

        let req = make_request(PlacementPolicy::GpuAffinity);
        let result = place(&req, &nodes).unwrap();
        assert_eq!(result.node_id, "b");
    }

    #[test]
    fn gpu_affinity_excludes_non_gpu_when_required() {
        let n1 = cpu_node("cpu-only");
        let n2 = gpu_node("gpu-box", 8000);
        let nodes: Vec<&NodeInfo> = vec![&n1, &n2];

        let req = PlacementRequest {
            policy: PlacementPolicy::GpuAffinity,
            required_gpu: true,
            min_gpu_vram_mb: 0,
            required_capabilities: Vec::new(),
            preferred_node: None,
            hardware: None,
        };
        let result = place(&req, &nodes).unwrap();
        assert_eq!(result.node_id, "gpu-box");

        // CPU-only node should not appear at all.
        let ranked = rank_nodes(&req, &nodes);
        assert!(ranked.iter().all(|r| r.node_id != "cpu-only"));
    }

    #[test]
    fn balanced_distributes_across_nodes() {
        let n1 = cpu_node("a");
        let n2 = cpu_node("b");
        let n3 = cpu_node("c");
        let nodes: Vec<&NodeInfo> = vec![&n1, &n2, &n3];

        let req = make_request(PlacementPolicy::Balanced);
        let ranked = rank_nodes(&req, &nodes);
        assert_eq!(ranked.len(), 3);
        // First node should have highest score.
        assert_eq!(ranked[0].node_id, "a");
        assert!(ranked[0].score > ranked[1].score);
        assert!(ranked[1].score > ranked[2].score);
    }

    #[test]
    fn locality_matches_capabilities() {
        let n1 = cpu_node("a").with_capabilities(vec!["python".into(), "docker".into()]);
        let n2 = cpu_node("b").with_capabilities(vec!["python".into()]);
        let n3 = cpu_node("c").with_capabilities(vec!["rust".into()]);
        let nodes: Vec<&NodeInfo> = vec![&n1, &n2, &n3];

        let req = PlacementRequest {
            policy: PlacementPolicy::Locality,
            required_gpu: false,
            min_gpu_vram_mb: 0,
            required_capabilities: vec!["python".into(), "docker".into()],
            preferred_node: None,
            hardware: None,
        };
        let result = place(&req, &nodes).unwrap();
        assert_eq!(result.node_id, "a");
        assert!((result.score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn cost_picks_smallest_node() {
        let n1 = gpu_node("big", 48000);
        let n2 = cpu_node("tiny");
        let n3 = gpu_node("medium", 16000);
        let nodes: Vec<&NodeInfo> = vec![&n1, &n2, &n3];

        let req = make_request(PlacementPolicy::Cost);
        let result = place(&req, &nodes).unwrap();
        assert_eq!(result.node_id, "tiny");
    }

    #[test]
    fn manual_returns_preferred_node() {
        let n1 = cpu_node("a");
        let n2 = cpu_node("target");
        let nodes: Vec<&NodeInfo> = vec![&n1, &n2];

        let req = PlacementRequest {
            policy: PlacementPolicy::Manual,
            required_gpu: false,
            min_gpu_vram_mb: 0,
            required_capabilities: Vec::new(),
            preferred_node: Some("target".into()),
            hardware: None,
        };
        let result = place(&req, &nodes).unwrap();
        assert_eq!(result.node_id, "target");
    }

    #[test]
    fn manual_returns_none_if_offline() {
        let n1 = cpu_node("target").with_status(NodeStatus::Offline);
        let nodes: Vec<&NodeInfo> = vec![&n1];

        let req = PlacementRequest {
            policy: PlacementPolicy::Manual,
            required_gpu: false,
            min_gpu_vram_mb: 0,
            required_capabilities: Vec::new(),
            preferred_node: Some("target".into()),
            hardware: None,
        };
        assert!(place(&req, &nodes).is_none());
    }

    #[test]
    fn no_online_nodes_returns_none() {
        let n1 = cpu_node("a").with_status(NodeStatus::Offline);
        let n2 = cpu_node("b").with_status(NodeStatus::Draining);
        let nodes: Vec<&NodeInfo> = vec![&n1, &n2];

        let req = make_request(PlacementPolicy::Balanced);
        assert!(place(&req, &nodes).is_none());
    }

    #[test]
    fn rank_nodes_returns_sorted() {
        let n1 = gpu_node("small", 4000);
        let n2 = gpu_node("big", 48000);
        let n3 = gpu_node("medium", 16000);
        let nodes: Vec<&NodeInfo> = vec![&n1, &n2, &n3];

        let req = make_request(PlacementPolicy::GpuAffinity);
        let ranked = rank_nodes(&req, &nodes);
        assert_eq!(ranked.len(), 3);
        assert_eq!(ranked[0].node_id, "big");
        assert_eq!(ranked[1].node_id, "medium");
        assert_eq!(ranked[2].node_id, "small");

        // Scores should be descending.
        for w in ranked.windows(2) {
            assert!(w[0].score >= w[1].score);
        }
    }

    #[test]
    fn hardware_cuda_requirement_filters_nodes() {
        use crate::core::resource::{
            AcceleratorType, ComputeDevice, HardwareInventory, HardwareRequirement,
        };

        let cuda_inventory = HardwareInventory {
            cpu_cores: 16,
            memory_total_mb: 65536,
            devices: vec![ComputeDevice {
                index: 0,
                name: "A100".into(),
                accelerator: AcceleratorType::Cuda,
                memory_total_mb: 81920,
                memory_available_mb: 81920,
            }],
        };
        let n1 = cpu_node("cpu-only");
        let n2 = gpu_node("cuda-box", 81920).with_hardware(cuda_inventory);
        let nodes: Vec<&NodeInfo> = vec![&n1, &n2];

        let req = PlacementRequest {
            policy: PlacementPolicy::Balanced,
            required_gpu: false,
            min_gpu_vram_mb: 0,
            required_capabilities: Vec::new(),
            preferred_node: None,
            hardware: Some(HardwareRequirement {
                accelerators: vec![AcceleratorType::Cuda],
                min_memory_mb: 40960,
                min_device_count: 1,
                min_cpu_cores: 0,
            required_family: None,
            }),
        };
        let result = place(&req, &nodes).unwrap();
        assert_eq!(result.node_id, "cuda-box");

        // CPU-only node should be filtered out.
        let ranked = rank_nodes(&req, &nodes);
        assert_eq!(ranked.len(), 1);
    }

    #[test]
    fn hardware_tpu_requirement_returns_none_when_no_tpu() {
        use crate::core::resource::{AcceleratorType, HardwareRequirement};

        let n1 = gpu_node("gpu-1", 24000);
        let n2 = cpu_node("cpu-1");
        let nodes: Vec<&NodeInfo> = vec![&n1, &n2];

        let req = PlacementRequest {
            policy: PlacementPolicy::Balanced,
            required_gpu: false,
            min_gpu_vram_mb: 0,
            required_capabilities: Vec::new(),
            preferred_node: None,
            hardware: Some(HardwareRequirement {
                accelerators: vec![AcceleratorType::Tpu],
                min_memory_mb: 0,
                min_device_count: 1,
                min_cpu_cores: 0,
            required_family: None,
            }),
        };
        assert!(place(&req, &nodes).is_none());
    }

    #[test]
    fn no_hardware_requirement_backward_compat() {
        // When hardware is None, legacy GPU fields still work.
        let n1 = gpu_node("gpu-box", 16000);
        let n2 = cpu_node("cpu-only");
        let nodes: Vec<&NodeInfo> = vec![&n1, &n2];

        let req = PlacementRequest {
            policy: PlacementPolicy::GpuAffinity,
            required_gpu: true,
            min_gpu_vram_mb: 8000,
            required_capabilities: Vec::new(),
            preferred_node: None,
            hardware: None,
        };
        let result = place(&req, &nodes).unwrap();
        assert_eq!(result.node_id, "gpu-box");
    }
}
