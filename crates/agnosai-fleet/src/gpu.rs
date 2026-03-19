//! GPU detection, VRAM tracking, and scheduling.

use std::collections::HashMap;
use uuid::Uuid;

/// Device index identifier.
pub type GpuId = usize;

/// A single GPU device with VRAM tracking.
#[derive(Debug, Clone)]
pub struct GpuDevice {
    pub index: GpuId,
    pub name: String,
    pub vram_total_mb: u64,
    pub vram_used_mb: u64,
}

impl GpuDevice {
    /// Available VRAM in megabytes.
    pub fn vram_available_mb(&self) -> u64 {
        self.vram_total_mb.saturating_sub(self.vram_used_mb)
    }
}

/// A VRAM allocation tied to a task.
#[derive(Debug, Clone)]
pub struct GpuAllocation {
    pub task_id: Uuid,
    pub device_index: GpuId,
    pub vram_mb: u64,
}

/// GPU scheduler managing device inventory and VRAM allocations.
pub struct GpuScheduler {
    devices: Vec<GpuDevice>,
    allocations: HashMap<Uuid, GpuAllocation>,
}

impl GpuScheduler {
    /// Create a new empty scheduler.
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            allocations: HashMap::new(),
        }
    }

    /// Add a GPU device to the scheduler.
    pub fn add_device(&mut self, name: String, vram_total_mb: u64) {
        let index = self.devices.len();
        self.devices.push(GpuDevice {
            index,
            name,
            vram_total_mb,
            vram_used_mb: 0,
        });
    }

    /// Get all registered devices.
    pub fn devices(&self) -> &[GpuDevice] {
        &self.devices
    }

    /// Total VRAM across all devices.
    pub fn total_vram_mb(&self) -> u64 {
        self.devices.iter().map(|d| d.vram_total_mb).sum()
    }

    /// Available (unused) VRAM across all devices.
    pub fn available_vram_mb(&self) -> u64 {
        self.devices.iter().map(|d| d.vram_available_mb()).sum()
    }

    /// Allocate VRAM on the best device for a task.
    ///
    /// Picks the device with the most available VRAM that has enough for the
    /// request. Returns `None` if no device can satisfy the allocation.
    pub fn allocate(&mut self, task_id: Uuid, vram_mb: u64) -> Option<GpuAllocation> {
        // Find device with most available VRAM that can satisfy the request.
        let best_idx = self
            .devices
            .iter()
            .filter(|d| d.vram_available_mb() >= vram_mb)
            .max_by_key(|d| d.vram_available_mb())
            .map(|d| d.index)?;

        self.devices[best_idx].vram_used_mb += vram_mb;

        let alloc = GpuAllocation {
            task_id,
            device_index: best_idx,
            vram_mb,
        };
        self.allocations.insert(task_id, alloc.clone());
        Some(alloc)
    }

    /// Release a previous allocation, returning VRAM to the device.
    ///
    /// Returns `true` if the allocation existed and was released.
    pub fn release(&mut self, task_id: Uuid) -> bool {
        if let Some(alloc) = self.allocations.remove(&task_id) {
            self.devices[alloc.device_index].vram_used_mb =
                self.devices[alloc.device_index]
                    .vram_used_mb
                    .saturating_sub(alloc.vram_mb);
            true
        } else {
            false
        }
    }

    /// Find the device with the most available VRAM.
    pub fn best_device(&self) -> Option<&GpuDevice> {
        self.devices.iter().max_by_key(|d| d.vram_available_mb())
    }

    /// List all active allocations.
    pub fn allocations(&self) -> Vec<&GpuAllocation> {
        self.allocations.values().collect()
    }
}

impl Default for GpuScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scheduler_with_devices() -> GpuScheduler {
        let mut s = GpuScheduler::new();
        s.add_device("RTX 4090".into(), 24000);
        s.add_device("A100".into(), 80000);
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
        let mut s = GpuScheduler::new();
        let id = Uuid::new_v4();
        assert!(s.allocate(id, 1000).is_none());
    }

    #[test]
    fn allocate_picks_most_available_vram() {
        let mut s = scheduler_with_devices();
        let id = Uuid::new_v4();
        let alloc = s.allocate(id, 10000).unwrap();
        // A100 (80 GB) has more VRAM than RTX 4090 (24 GB).
        assert_eq!(alloc.device_index, 1);
        assert_eq!(alloc.vram_mb, 10000);
    }

    #[test]
    fn allocate_reduces_available_vram() {
        let mut s = scheduler_with_devices();
        let id = Uuid::new_v4();
        s.allocate(id, 10000).unwrap();
        assert_eq!(s.available_vram_mb(), 94_000);
    }

    #[test]
    fn allocate_fails_when_insufficient_vram() {
        let mut s = scheduler_with_devices();
        // Try to allocate more than any single device has.
        let id = Uuid::new_v4();
        assert!(s.allocate(id, 100_000).is_none());
    }

    #[test]
    fn release_restores_vram() {
        let mut s = scheduler_with_devices();
        let id = Uuid::new_v4();
        s.allocate(id, 20000).unwrap();
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
        let a1 = s.allocate(id1, 60000).unwrap();
        assert_eq!(a1.device_index, 1); // A100

        // Second alloc: A100 has 20 GB left, RTX 4090 has 24 GB — picks 4090.
        let a2 = s.allocate(id2, 20000).unwrap();
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
        s.allocate(id, 70000).unwrap();

        // Now RTX 4090 (24 GB free) > A100 (10 GB free).
        assert_eq!(s.best_device().unwrap().name, "RTX 4090");
    }
}
