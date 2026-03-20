//! Benchmarks for hardware inventory operations.

use criterion::{Criterion, criterion_group, criterion_main};

use agnosai::core::resource::*;

fn small_inventory() -> HardwareInventory {
    HardwareInventory {
        cpu_cores: 8,
        memory_total_mb: 32768,
        devices: vec![ComputeDevice {
            index: 0,
            name: "NVIDIA A100".into(),
            accelerator: AcceleratorType::Cuda,
            memory_total_mb: 81920,
            memory_available_mb: 40960,
        }],
    }
}

fn large_inventory() -> HardwareInventory {
    let mut devices = Vec::new();
    for i in 0..8 {
        devices.push(ComputeDevice {
            index: i,
            name: format!("NVIDIA H100 #{i}"),
            accelerator: AcceleratorType::Cuda,
            memory_total_mb: 81920,
            memory_available_mb: 81920,
        });
    }
    devices.push(ComputeDevice {
        index: 8,
        name: "TPU v4".into(),
        accelerator: AcceleratorType::Tpu,
        memory_total_mb: 32768,
        memory_available_mb: 32768,
    });
    devices.push(ComputeDevice {
        index: 9,
        name: "AMD MI300X".into(),
        accelerator: AcceleratorType::Rocm,
        memory_total_mb: 196608,
        memory_available_mb: 196608,
    });
    HardwareInventory {
        cpu_cores: 128,
        memory_total_mb: 1048576,
        devices,
    }
}

fn bench_satisfies(c: &mut Criterion) {
    let inv = large_inventory();
    let req = HardwareRequirement {
        accelerators: vec![AcceleratorType::Cuda],
        min_memory_mb: 40960,
        min_device_count: 4,
        min_cpu_cores: 64,
    };
    c.bench_function("satisfies (10 devices, CUDA req)", |b| {
        b.iter(|| inv.satisfies(&req));
    });
}

fn bench_satisfies_empty_req(c: &mut Criterion) {
    let inv = large_inventory();
    let req = HardwareRequirement::default();
    c.bench_function("satisfies (10 devices, empty req)", |b| {
        b.iter(|| inv.satisfies(&req));
    });
}

fn bench_devices_of_type(c: &mut Criterion) {
    let inv = large_inventory();
    c.bench_function("devices_of_type CUDA (10 devices)", |b| {
        b.iter(|| inv.devices_of_type(AcceleratorType::Cuda));
    });
}

fn bench_total_memory_mb(c: &mut Criterion) {
    let inv = large_inventory();
    c.bench_function("total_memory_mb CUDA (10 devices)", |b| {
        b.iter(|| inv.total_memory_mb(AcceleratorType::Cuda));
    });
}

fn bench_from_hwaccel(c: &mut Criterion) {
    use ai_hwaccel::{AcceleratorProfile, AcceleratorRegistry};

    let registry = AcceleratorRegistry::from_profiles(vec![
        AcceleratorProfile::cpu(512 * 1024 * 1024 * 1024),
        AcceleratorProfile::cuda(0, 80 * 1024 * 1024 * 1024),
        AcceleratorProfile::cuda(1, 80 * 1024 * 1024 * 1024),
        AcceleratorProfile::cuda(2, 80 * 1024 * 1024 * 1024),
        AcceleratorProfile::cuda(3, 80 * 1024 * 1024 * 1024),
        AcceleratorProfile::cuda(4, 80 * 1024 * 1024 * 1024),
        AcceleratorProfile::cuda(5, 80 * 1024 * 1024 * 1024),
        AcceleratorProfile::cuda(6, 80 * 1024 * 1024 * 1024),
        AcceleratorProfile::cuda(7, 80 * 1024 * 1024 * 1024),
    ]);
    c.bench_function("from_hwaccel (8 GPUs + CPU)", |b| {
        b.iter(|| HardwareInventory::from_hwaccel(&registry));
    });
}

fn bench_satisfies_small(c: &mut Criterion) {
    let inv = small_inventory();
    let req = HardwareRequirement {
        accelerators: vec![AcceleratorType::Cuda],
        min_memory_mb: 40000,
        min_device_count: 1,
        min_cpu_cores: 4,
    };
    c.bench_function("satisfies (1 device, CUDA req)", |b| {
        b.iter(|| inv.satisfies(&req));
    });
}

criterion_group!(
    benches,
    bench_satisfies,
    bench_satisfies_empty_req,
    bench_satisfies_small,
    bench_devices_of_type,
    bench_total_memory_mb,
    bench_from_hwaccel,
);
criterion_main!(benches);
