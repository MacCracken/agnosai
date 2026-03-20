//! Node inventory with heartbeat and TTL-based liveness.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::core::resource::{HardwareInventory, HardwareRequirement};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a fleet node.
pub type NodeId = String;

/// Status of a fleet node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum NodeStatus {
    Online,
    /// Missed heartbeat but within grace period.
    Suspect,
    Offline,
    Draining,
}

/// Information about a single fleet node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub id: NodeId,
    pub hostname: String,
    /// Network address in `"host:port"` format.
    pub address: String,
    pub status: NodeStatus,
    pub gpu_count: u32,
    pub gpu_vram_mb: u64,
    pub capabilities: Vec<String>,
    /// Hardware inventory for this node.
    #[serde(default)]
    pub hardware: HardwareInventory,
    pub last_heartbeat: DateTime<Utc>,
    /// Monotonic instant of registration (not serialized).
    #[serde(skip, default = "Instant::now")]
    pub registered_at: Instant,
    /// Monotonic instant of last heartbeat (not serialized).
    #[serde(skip, default = "Instant::now")]
    pub last_heartbeat_instant: Instant,
}

impl NodeInfo {
    /// Create a new node with the given parameters.
    pub fn new(id: impl Into<String>, gpu_count: u32, gpu_vram_mb: u64) -> Self {
        let id = id.into();
        Self {
            hostname: id.clone(),
            address: String::new(),
            id,
            status: NodeStatus::Online,
            gpu_count,
            gpu_vram_mb,
            capabilities: Vec::new(),
            hardware: HardwareInventory::default(),
            last_heartbeat: Utc::now(),
            registered_at: Instant::now(),
            last_heartbeat_instant: Instant::now(),
        }
    }

    /// Builder-style method to set hardware inventory.
    pub fn with_hardware(mut self, hardware: HardwareInventory) -> Self {
        self.hardware = hardware;
        self
    }

    /// Check if this node's hardware satisfies a requirement.
    pub fn satisfies_hardware(&self, req: &HardwareRequirement) -> bool {
        self.hardware.satisfies(req)
    }

    /// Builder-style method to set capabilities.
    pub fn with_capabilities(mut self, caps: Vec<String>) -> Self {
        self.capabilities = caps;
        self
    }

    /// Builder-style method to set status.
    pub fn with_status(mut self, status: NodeStatus) -> Self {
        self.status = status;
        self
    }

    /// Whether this node has any GPU.
    pub fn has_gpu(&self) -> bool {
        self.gpu_count > 0
    }
}

/// In-memory node registry with heartbeat tracking and TTL-based status transitions.
pub struct NodeRegistry {
    nodes: HashMap<NodeId, NodeInfo>,
    /// Duration after which a node becomes `Suspect` (default 30s).
    heartbeat_ttl: Duration,
    /// Duration after which a node becomes `Offline` (default 90s).
    offline_ttl: Duration,
}

impl NodeRegistry {
    /// Create a registry with default TTLs (30s heartbeat, 90s offline).
    pub fn new() -> Self {
        Self::with_ttl(Duration::from_secs(30), Duration::from_secs(90))
    }

    /// Create a registry with custom TTLs.
    pub fn with_ttl(heartbeat_ttl: Duration, offline_ttl: Duration) -> Self {
        Self {
            nodes: HashMap::new(),
            heartbeat_ttl,
            offline_ttl,
        }
    }

    /// Register a new node. Returns the assigned `NodeId`.
    pub fn register(
        &mut self,
        hostname: String,
        address: String,
        gpu_count: usize,
        gpu_vram_mb: u64,
        capabilities: Vec<String>,
    ) -> NodeId {
        let id = Uuid::new_v4().to_string();
        let now = Instant::now();
        let info = NodeInfo {
            id: id.clone(),
            hostname,
            address,
            status: NodeStatus::Online,
            gpu_count: gpu_count as u32,
            gpu_vram_mb,
            capabilities,
            hardware: HardwareInventory::default(),
            last_heartbeat: Utc::now(),
            registered_at: now,
            last_heartbeat_instant: now,
        };
        self.nodes.insert(id.clone(), info);
        id
    }

    /// Record a heartbeat for a node. Returns `false` if the node is unknown.
    pub fn heartbeat(&mut self, node_id: NodeId) -> bool {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.last_heartbeat_instant = Instant::now();
            node.last_heartbeat = Utc::now();
            node.status = NodeStatus::Online;
            true
        } else {
            false
        }
    }

    /// Remove a node from the registry. Returns `true` if it existed.
    pub fn unregister(&mut self, node_id: NodeId) -> bool {
        self.nodes.remove(&node_id).is_some()
    }

    /// Look up a node by ID.
    pub fn get(&self, node_id: &str) -> Option<&NodeInfo> {
        self.nodes.get(node_id)
    }

    /// List all registered nodes.
    pub fn list(&self) -> Vec<&NodeInfo> {
        self.nodes.values().collect()
    }

    /// List only nodes with `Online` status.
    pub fn list_online(&self) -> Vec<&NodeInfo> {
        self.nodes
            .values()
            .filter(|n| n.status == NodeStatus::Online)
            .collect()
    }

    /// Sweep all nodes and update statuses based on heartbeat TTLs.
    pub fn update_statuses(&mut self) {
        let now = Instant::now();
        for node in self.nodes.values_mut() {
            let elapsed = now.duration_since(node.last_heartbeat_instant);
            if elapsed >= self.offline_ttl {
                node.status = NodeStatus::Offline;
            } else if elapsed >= self.heartbeat_ttl {
                node.status = NodeStatus::Suspect;
            }
        }
    }

    /// Total number of registered nodes (any status).
    pub fn count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of nodes currently `Online`.
    pub fn count_online(&self) -> usize {
        self.nodes
            .values()
            .filter(|n| n.status == NodeStatus::Online)
            .count()
    }

    /// Find online nodes that advertise a given capability.
    pub fn find_by_capability(&self, capability: &str) -> Vec<&NodeInfo> {
        self.nodes
            .values()
            .filter(|n| {
                n.status == NodeStatus::Online && n.capabilities.iter().any(|c| c == capability)
            })
            .collect()
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_retrieve() {
        let mut reg = NodeRegistry::new();
        let id = reg.register(
            "node-1".into(),
            "10.0.0.1:8080".into(),
            2,
            16384,
            vec!["inference".into()],
        );

        let info = reg.get(&id).expect("node should exist");
        assert_eq!(info.hostname, "node-1");
        assert_eq!(info.address, "10.0.0.1:8080");
        assert_eq!(info.gpu_count, 2);
        assert_eq!(info.gpu_vram_mb, 16384);
        assert_eq!(info.capabilities, vec!["inference"]);
        assert_eq!(info.status, NodeStatus::Online);
    }

    #[test]
    fn heartbeat_updates_timestamp() {
        let mut reg = NodeRegistry::new();
        let id = reg.register("n".into(), "addr".into(), 0, 0, vec![]);

        let before = reg.get(&id).unwrap().last_heartbeat_instant;
        std::thread::sleep(Duration::from_millis(5));
        assert!(reg.heartbeat(id.clone()));
        let after = reg.get(&id).unwrap().last_heartbeat_instant;
        assert!(after > before);
    }

    #[test]
    fn heartbeat_unknown_node_returns_false() {
        let mut reg = NodeRegistry::new();
        assert!(!reg.heartbeat("nonexistent".into()));
    }

    #[test]
    fn status_transitions() {
        let mut reg = NodeRegistry::with_ttl(Duration::from_millis(20), Duration::from_millis(60));
        let id = reg.register("n".into(), "a".into(), 0, 0, vec![]);

        assert_eq!(reg.get(&id).unwrap().status, NodeStatus::Online);

        // Wait past heartbeat TTL but before offline TTL.
        std::thread::sleep(Duration::from_millis(30));
        reg.update_statuses();
        assert_eq!(reg.get(&id).unwrap().status, NodeStatus::Suspect);

        // Wait past offline TTL.
        std::thread::sleep(Duration::from_millis(40));
        reg.update_statuses();
        assert_eq!(reg.get(&id).unwrap().status, NodeStatus::Offline);
    }

    #[test]
    fn heartbeat_resets_to_online() {
        let mut reg = NodeRegistry::with_ttl(Duration::from_millis(10), Duration::from_millis(50));
        let id = reg.register("n".into(), "a".into(), 0, 0, vec![]);

        std::thread::sleep(Duration::from_millis(15));
        reg.update_statuses();
        assert_eq!(reg.get(&id).unwrap().status, NodeStatus::Suspect);

        assert!(reg.heartbeat(id.clone()));
        assert_eq!(reg.get(&id).unwrap().status, NodeStatus::Online);
    }

    #[test]
    fn unregister_removes_node() {
        let mut reg = NodeRegistry::new();
        let id = reg.register("n".into(), "a".into(), 0, 0, vec![]);
        assert!(reg.unregister(id.clone()));
        assert!(reg.get(&id).is_none());
        assert!(!reg.unregister(id));
    }

    #[test]
    fn list_online_filters_correctly() {
        let mut reg = NodeRegistry::with_ttl(Duration::from_millis(10), Duration::from_millis(50));
        let id1 = reg.register("a".into(), "a".into(), 0, 0, vec![]);
        let _id2 = reg.register("b".into(), "b".into(), 0, 0, vec![]);

        std::thread::sleep(Duration::from_millis(15));
        reg.update_statuses();
        assert_eq!(reg.count_online(), 0);

        reg.heartbeat(id1.clone());
        assert_eq!(reg.count_online(), 1);
        assert_eq!(reg.list_online().len(), 1);
        assert_eq!(reg.list_online()[0].id, id1);
    }

    #[test]
    fn find_by_capability_works() {
        let mut reg = NodeRegistry::new();
        reg.register(
            "a".into(),
            "a".into(),
            1,
            8192,
            vec!["inference".into(), "training".into()],
        );
        reg.register("b".into(), "b".into(), 0, 0, vec!["inference".into()]);
        reg.register("c".into(), "c".into(), 0, 0, vec!["storage".into()]);

        let inf = reg.find_by_capability("inference");
        assert_eq!(inf.len(), 2);

        let train = reg.find_by_capability("training");
        assert_eq!(train.len(), 1);

        let none = reg.find_by_capability("nonexistent");
        assert!(none.is_empty());
    }

    #[test]
    fn count_and_count_online() {
        let mut reg = NodeRegistry::with_ttl(Duration::from_millis(10), Duration::from_millis(50));
        assert_eq!(reg.count(), 0);
        assert_eq!(reg.count_online(), 0);

        reg.register("a".into(), "a".into(), 0, 0, vec![]);
        reg.register("b".into(), "b".into(), 0, 0, vec![]);
        assert_eq!(reg.count(), 2);
        assert_eq!(reg.count_online(), 2);

        std::thread::sleep(Duration::from_millis(15));
        reg.update_statuses();
        assert_eq!(reg.count(), 2);
        assert_eq!(reg.count_online(), 0);
    }

    // Backward compat: NodeInfo::new and builder methods still work.
    #[test]
    fn node_info_builder_compat() {
        let node = NodeInfo::new("test-node", 1, 8192)
            .with_capabilities(vec!["python".into()])
            .with_status(NodeStatus::Draining);
        assert_eq!(node.id, "test-node");
        assert_eq!(node.gpu_count, 1);
        assert_eq!(node.gpu_vram_mb, 8192);
        assert_eq!(node.capabilities, vec!["python"]);
        assert_eq!(node.status, NodeStatus::Draining);
        assert!(node.has_gpu());
    }

    #[test]
    fn satisfies_hardware_with_inventory() {
        use crate::core::resource::{
            AcceleratorType, ComputeDevice, HardwareInventory, HardwareRequirement,
        };

        let inventory = HardwareInventory {
            cpu_cores: 16,
            memory_total_mb: 65536,
            devices: vec![
                ComputeDevice {
                    index: 0,
                    name: "NVIDIA A100".into(),
                    accelerator: AcceleratorType::Cuda,
                    memory_total_mb: 81920,
                    memory_available_mb: 81920,
                },
                ComputeDevice {
                    index: 1,
                    name: "NVIDIA A100".into(),
                    accelerator: AcceleratorType::Cuda,
                    memory_total_mb: 81920,
                    memory_available_mb: 81920,
                },
            ],
        };

        let node = NodeInfo::new("hw-node", 2, 81920).with_hardware(inventory);

        // Should satisfy CUDA requirement.
        let cuda_req = HardwareRequirement {
            accelerators: vec![AcceleratorType::Cuda],
            min_memory_mb: 40960,
            min_device_count: 2,
            min_cpu_cores: 8,
            required_family: None,
        };
        assert!(node.satisfies_hardware(&cuda_req));

        // Should NOT satisfy TPU requirement.
        let tpu_req = HardwareRequirement {
            accelerators: vec![AcceleratorType::Tpu],
            min_memory_mb: 0,
            min_device_count: 1,
            min_cpu_cores: 0,
            required_family: None,
        };
        assert!(!node.satisfies_hardware(&tpu_req));

        // Empty requirement should always pass.
        let empty_req = HardwareRequirement::default();
        assert!(node.satisfies_hardware(&empty_req));
    }

    #[test]
    fn node_info_default_hardware_is_empty() {
        let node = NodeInfo::new("plain", 0, 0);
        assert!(node.hardware.devices.is_empty());
        assert_eq!(node.hardware.cpu_cores, 0);
    }
}
