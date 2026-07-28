//! Benchmarks for hardware inventory operations.

use criterion::{Criterion, criterion_group, criterion_main};

use agnosai::core::resource::*;

fn small_inventory() -> HardwareInventory {
    HardwareInventory::new(
        8,
        32768,
        vec![
            ComputeDevice::new(0, "NVIDIA A100", AcceleratorType::Cuda, 81920)
                .with_available(40960),
        ],
    )
}

fn large_inventory() -> HardwareInventory {
    let mut devices: Vec<ComputeDevice> = (0..8)
        .map(|i| ComputeDevice::new(i, format!("NVIDIA H100 #{i}"), AcceleratorType::Cuda, 81920))
        .collect();
    devices.push(ComputeDevice::new(8, "TPU v4", AcceleratorType::Tpu, 32768));
    devices.push(ComputeDevice::new(
        9,
        "AMD MI300X",
        AcceleratorType::Rocm,
        196608,
    ));
    HardwareInventory::new(128, 1048576, devices)
}

fn bench_satisfies(c: &mut Criterion) {
    let inv = large_inventory();
    let req = HardwareRequirement::for_accelerators(vec![AcceleratorType::Cuda])
        .with_min_memory(40960)
        .with_min_devices(4)
        .with_min_cpu_cores(64);
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
    let req = HardwareRequirement::for_accelerators(vec![AcceleratorType::Cuda])
        .with_min_memory(40000)
        .with_min_devices(1)
        .with_min_cpu_cores(4);
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
