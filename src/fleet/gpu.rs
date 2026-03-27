//! Compute device scheduling — manages allocation of accelerator memory (GPU VRAM, TPU HBM, etc).

use std::collections::HashMap;

use crate::core::resource::{AcceleratorType, ComputeDevice};
use uuid::Uuid;

/// Device index identifier.
pub type GpuId = usize;

/// A VRAM allocation tied to a task (kept for backward compat).
pub type GpuAllocation = ComputeAllocation;

/// A compute memory allocation tied to a task.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ComputeAllocation {
    pub task_id: Uuid,
    pub device_index: usize,
    /// Allocated memory in MB (VRAM, HBM, etc).
    pub memory_mb: u64,
}

// Backward compat: expose vram_mb as an alias.
impl ComputeAllocation {
    /// Alias for `memory_mb` (backward compatibility).
    pub fn vram_mb(&self) -> u64 {
        self.memory_mb
    }
}

/// Internal device state tracking used memory.
#[derive(Debug, Clone)]
struct DeviceState {
    device: ComputeDevice,
    memory_used_mb: u64,
}

impl DeviceState {
    fn memory_available_mb(&self) -> u64 {
        self.device
            .memory_total_mb
            .saturating_sub(self.memory_used_mb)
    }
}

/// Compute device scheduler — manages allocation of accelerator memory (GPU VRAM, TPU HBM, etc).
pub struct ComputeScheduler {
    devices: Vec<DeviceState>,
    allocations: HashMap<Uuid, ComputeAllocation>,
}

/// Backward compatibility type alias.
pub type GpuScheduler = ComputeScheduler;

/// Legacy GPU device view (read-only).
///
/// Prefer [`crate::core::resource::GpuDevice`] for serializable GPU state.
/// This type exists for backward compatibility with fleet scheduling internals.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct GpuDevice {
    pub index: GpuId,
    pub name: String,
    pub vram_total_mb: u64,
    pub vram_used_mb: u64,
}

impl GpuDevice {
    /// Available VRAM in megabytes.
    #[must_use]
    pub fn vram_available_mb(&self) -> u64 {
        self.vram_total_mb.saturating_sub(self.vram_used_mb)
    }
}

impl ComputeScheduler {
    /// Create a new empty scheduler.
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            allocations: HashMap::new(),
        }
    }

    /// Add a compute device with the specified accelerator type.
    pub fn add_device(&mut self, name: String, accelerator: AcceleratorType, memory_total_mb: u64) {
        let index = self.devices.len();
        self.devices.push(DeviceState {
            device: ComputeDevice {
                index,
                name,
                accelerator,
                memory_total_mb,
                memory_available_mb: memory_total_mb,
            },
            memory_used_mb: 0,
        });
    }

    /// Add a GPU device (backward compat — defaults to CUDA).
    pub fn add_gpu(&mut self, name: String, vram_total_mb: u64) {
        self.add_device(name, AcceleratorType::Cuda, vram_total_mb);
    }

    /// Get all registered devices as `ComputeDevice` references.
    #[must_use]
    pub fn compute_devices(&self) -> Vec<&ComputeDevice> {
        self.devices.iter().map(|ds| &ds.device).collect()
    }

    /// Get all registered devices as legacy `GpuDevice` views.
    #[must_use]
    pub fn devices(&self) -> Vec<GpuDevice> {
        self.devices
            .iter()
            .map(|ds| GpuDevice {
                index: ds.device.index,
                name: ds.device.name.clone(),
                vram_total_mb: ds.device.memory_total_mb,
                vram_used_mb: ds.memory_used_mb,
            })
            .collect()
    }

    /// Get devices of a specific accelerator type.
    #[must_use]
    pub fn devices_of_type(&self, accel: AcceleratorType) -> Vec<&ComputeDevice> {
        self.devices
            .iter()
            .filter(|ds| ds.device.accelerator == accel)
            .map(|ds| &ds.device)
            .collect()
    }

    /// Total memory across all devices, optionally filtered by accelerator type.
    #[must_use]
    pub fn total_memory_mb(&self, accel: Option<AcceleratorType>) -> u64 {
        self.devices
            .iter()
            .filter(|ds| accel.is_none_or(|a| ds.device.accelerator == a))
            .map(|ds| ds.device.memory_total_mb)
            .sum()
    }

    /// Available (unused) memory across all devices, optionally filtered by accelerator type.
    #[must_use]
    pub fn available_memory_mb(&self, accel: Option<AcceleratorType>) -> u64 {
        self.devices
            .iter()
            .filter(|ds| accel.is_none_or(|a| ds.device.accelerator == a))
            .map(|ds| ds.memory_available_mb())
            .sum()
    }

    /// Backward compat: total VRAM across all devices.
    #[must_use]
    pub fn total_vram_mb(&self) -> u64 {
        self.total_memory_mb(None)
    }

    /// Backward compat: available VRAM across all devices.
    #[must_use]
    pub fn available_vram_mb(&self) -> u64 {
        self.available_memory_mb(None)
    }

    /// Allocate memory on the best device of the specified type.
    /// If `accel` is None, picks any device with enough memory.
    pub fn allocate(
        &mut self,
        task_id: Uuid,
        memory_mb: u64,
        accel: Option<AcceleratorType>,
    ) -> Option<ComputeAllocation> {
        // Find device with most available memory that can satisfy the request.
        let best_idx = self
            .devices
            .iter()
            .enumerate()
            .filter(|(_, ds)| accel.is_none_or(|a| ds.device.accelerator == a))
            .filter(|(_, ds)| ds.memory_available_mb() >= memory_mb)
            .max_by_key(|(_, ds)| ds.memory_available_mb())
            .map(|(i, _)| i)?;

        self.devices[best_idx].memory_used_mb += memory_mb;
        // Keep ComputeDevice.memory_available_mb in sync for external consumers.
        self.devices[best_idx].device.memory_available_mb =
            self.devices[best_idx].memory_available_mb();

        let alloc = ComputeAllocation {
            task_id,
            device_index: best_idx,
            memory_mb,
        };
        self.allocations.insert(task_id, alloc.clone());
        Some(alloc)
    }

    /// Release a previous allocation, returning memory to the device.
    /// Returns `true` if the allocation existed and was released.
    pub fn release(&mut self, task_id: Uuid) -> bool {
        if let Some(alloc) = self.allocations.remove(&task_id) {
            self.devices[alloc.device_index].memory_used_mb = self.devices[alloc.device_index]
                .memory_used_mb
                .saturating_sub(alloc.memory_mb);
            // Keep ComputeDevice.memory_available_mb in sync.
            self.devices[alloc.device_index].device.memory_available_mb =
                self.devices[alloc.device_index].memory_available_mb();
            true
        } else {
            false
        }
    }

    /// Find the device with the most available memory.
    #[must_use]
    pub fn best_device(&self) -> Option<GpuDevice> {
        self.devices
            .iter()
            .max_by_key(|ds| ds.memory_available_mb())
            .map(|ds| GpuDevice {
                index: ds.device.index,
                name: ds.device.name.clone(),
                vram_total_mb: ds.device.memory_total_mb,
                vram_used_mb: ds.memory_used_mb,
            })
    }

    /// List all active allocations.
    #[must_use]
    pub fn allocations(&self) -> Vec<&ComputeAllocation> {
        self.allocations.values().collect()
    }
}

impl Default for ComputeScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scheduler_with_devices() -> ComputeScheduler {
        let mut s = ComputeScheduler::new();
        s.add_device("RTX 4090".into(), AcceleratorType::Cuda, 24000);
        s.add_device("A100".into(), AcceleratorType::Cuda, 80000);
        s
    }

    #[test]
    fn add_devices_and_verify() {
        let s = scheduler_with_devices();
        assert_eq!(s.devices().len(), 2);
        assert_eq!(s.total_vram_mb(), 104_000);
        assert_eq!(s.available_vram_mb(), 104_000);
    }

    #[test]
    fn allocate_on_empty_returns_none() {
        let mut s = ComputeScheduler::new();
        let id = Uuid::new_v4();
        assert!(s.allocate(id, 1000, None).is_none());
    }

    #[test]
    fn allocate_picks_most_available_vram() {
        let mut s = scheduler_with_devices();
        let id = Uuid::new_v4();
        let alloc = s.allocate(id, 10000, None).unwrap();
        // A100 (80 GB) has more VRAM than RTX 4090 (24 GB).
        assert_eq!(alloc.device_index, 1);
        assert_eq!(alloc.memory_mb, 10000);
    }

    #[test]
    fn allocate_reduces_available_vram() {
        let mut s = scheduler_with_devices();
        let id = Uuid::new_v4();
        s.allocate(id, 10000, None).unwrap();
        assert_eq!(s.available_vram_mb(), 94_000);
    }

    #[test]
    fn allocate_fails_when_insufficient_vram() {
        let mut s = scheduler_with_devices();
        // Try to allocate more than any single device has.
        let id = Uuid::new_v4();
        assert!(s.allocate(id, 100_000, None).is_none());
    }

    #[test]
    fn release_restores_vram() {
        let mut s = scheduler_with_devices();
        let id = Uuid::new_v4();
        s.allocate(id, 20000, None).unwrap();
        assert_eq!(s.available_vram_mb(), 84_000);

        assert!(s.release(id));
        assert_eq!(s.available_vram_mb(), 104_000);

        // Releasing again returns false.
        assert!(!s.release(id));
    }

    #[test]
    fn multiple_allocations_on_same_device() {
        let mut s = scheduler_with_devices();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        // First alloc goes to A100 (most available).
        let a1 = s.allocate(id1, 60000, None).unwrap();
        assert_eq!(a1.device_index, 1); // A100

        // Second alloc: A100 has 20 GB left, RTX 4090 has 24 GB — picks 4090.
        let a2 = s.allocate(id2, 20000, None).unwrap();
        assert_eq!(a2.device_index, 0); // RTX 4090

        assert_eq!(s.allocations().len(), 2);
    }

    #[test]
    fn best_device_returns_correct() {
        let mut s = scheduler_with_devices();
        // Initially, A100 is best.
        assert_eq!(s.best_device().unwrap().name, "A100");

        // Allocate most of A100.
        let id = Uuid::new_v4();
        s.allocate(id, 70000, None).unwrap();

        // Now RTX 4090 (24 GB free) > A100 (10 GB free).
        assert_eq!(s.best_device().unwrap().name, "RTX 4090");
    }

    // --- New accelerator-aware tests ---

    #[test]
    fn add_cuda_and_tpu_devices() {
        let mut s = ComputeScheduler::new();
        s.add_device("A100".into(), AcceleratorType::Cuda, 80000);
        s.add_device("TPU v4".into(), AcceleratorType::Tpu, 32000);
        assert_eq!(s.compute_devices().len(), 2);
        assert_eq!(s.total_memory_mb(Some(AcceleratorType::Cuda)), 80000);
        assert_eq!(s.total_memory_mb(Some(AcceleratorType::Tpu)), 32000);
        assert_eq!(s.total_memory_mb(None), 112_000);
    }

    #[test]
    fn allocate_on_specific_accelerator_type() {
        let mut s = ComputeScheduler::new();
        s.add_device("A100".into(), AcceleratorType::Cuda, 80000);
        s.add_device("TPU v4".into(), AcceleratorType::Tpu, 32000);

        let id = Uuid::new_v4();
        let alloc = s.allocate(id, 16000, Some(AcceleratorType::Tpu)).unwrap();
        assert_eq!(alloc.device_index, 1); // TPU is at index 1
        assert_eq!(alloc.memory_mb, 16000);

        // TPU available should be reduced, CUDA unaffected.
        assert_eq!(s.available_memory_mb(Some(AcceleratorType::Tpu)), 16000);
        assert_eq!(s.available_memory_mb(Some(AcceleratorType::Cuda)), 80000);
    }

    #[test]
    fn allocate_tpu_memory() {
        let mut s = ComputeScheduler::new();
        s.add_device("TPU v4-a".into(), AcceleratorType::Tpu, 32000);
        s.add_device("TPU v4-b".into(), AcceleratorType::Tpu, 32000);

        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        s.allocate(id1, 24000, Some(AcceleratorType::Tpu)).unwrap();
        s.allocate(id2, 24000, Some(AcceleratorType::Tpu)).unwrap();

        assert_eq!(s.available_memory_mb(Some(AcceleratorType::Tpu)), 16000);

        // Can't fit another 24000 on either TPU.
        let id3 = Uuid::new_v4();
        assert!(s.allocate(id3, 24000, Some(AcceleratorType::Tpu)).is_none());
    }

    #[test]
    fn mixed_devices_allocate_prefers_requested_type() {
        let mut s = ComputeScheduler::new();
        // CUDA device has more memory, but we request TPU.
        s.add_device("A100".into(), AcceleratorType::Cuda, 80000);
        s.add_device("TPU v4".into(), AcceleratorType::Tpu, 32000);

        let id = Uuid::new_v4();
        let alloc = s.allocate(id, 16000, Some(AcceleratorType::Tpu)).unwrap();
        assert_eq!(alloc.device_index, 1); // TPU, not the bigger CUDA device
    }

    #[test]
    fn devices_of_type_filters_correctly() {
        let mut s = ComputeScheduler::new();
        s.add_device("A100-0".into(), AcceleratorType::Cuda, 80000);
        s.add_device("TPU v4".into(), AcceleratorType::Tpu, 32000);
        s.add_device("A100-1".into(), AcceleratorType::Cuda, 80000);

        let cuda = s.devices_of_type(AcceleratorType::Cuda);
        assert_eq!(cuda.len(), 2);
        assert!(cuda.iter().all(|d| d.accelerator == AcceleratorType::Cuda));

        let tpu = s.devices_of_type(AcceleratorType::Tpu);
        assert_eq!(tpu.len(), 1);
        assert_eq!(tpu[0].name, "TPU v4");

        let rocm = s.devices_of_type(AcceleratorType::Rocm);
        assert!(rocm.is_empty());
    }

    #[test]
    fn backward_compat_add_gpu() {
        let mut s = GpuScheduler::new();
        s.add_gpu("RTX 4090".into(), 24000);
        assert_eq!(s.devices().len(), 1);
        assert_eq!(s.devices()[0].vram_total_mb, 24000);
        assert_eq!(s.total_vram_mb(), 24000);
        assert_eq!(s.available_vram_mb(), 24000);

        // Allocate with None accel (backward compat pattern).
        let id = Uuid::new_v4();
        let alloc = s.allocate(id, 10000, None).unwrap();
        assert_eq!(alloc.memory_mb, 10000);
        assert_eq!(alloc.vram_mb(), 10000);
        assert_eq!(s.available_vram_mb(), 14000);
    }

    #[test]
    fn gpu_allocation_type_alias_works() {
        // GpuAllocation is an alias for ComputeAllocation.
        let alloc: GpuAllocation = ComputeAllocation {
            task_id: Uuid::new_v4(),
            device_index: 0,
            memory_mb: 8000,
        };
        assert_eq!(alloc.vram_mb(), 8000);
    }
}
