//! Benchmarks for the fleet placement engine.

use criterion::{Criterion, criterion_group, criterion_main};

use agnosai::core::resource::{
    AcceleratorType, ComputeDevice, HardwareInventory, HardwareRequirement,
};
use agnosai::fleet::placement::{PlacementPolicy, PlacementRequest, place, rank_nodes};
use agnosai::fleet::registry::NodeInfo;

fn gpu_node(id: &str, vram: u64) -> NodeInfo {
    NodeInfo::new(id, 1, vram)
}

fn cpu_node(id: &str) -> NodeInfo {
    NodeInfo::new(id, 0, 0)
}

fn gpu_node_with_hw(id: &str, num_gpus: usize, vram_per_gpu: u64) -> NodeInfo {
    let mut devices = Vec::new();
    for i in 0..num_gpus {
        devices.push(ComputeDevice {
            index: i,
            name: format!("GPU #{i}"),
            accelerator: AcceleratorType::Cuda,
            memory_total_mb: vram_per_gpu,
            memory_available_mb: vram_per_gpu,
        });
    }
    let inv = HardwareInventory {
        cpu_cores: 64,
        memory_total_mb: 524288,
        devices,
    };
    NodeInfo::new(id, num_gpus as u32, vram_per_gpu)
        .with_hardware(inv)
}

fn make_fleet(n: usize) -> Vec<NodeInfo> {
    (0..n)
        .map(|i| {
            if i % 3 == 0 {
                cpu_node(&format!("cpu-{i}"))
            } else {
                gpu_node_with_hw(&format!("gpu-{i}"), (i % 4) + 1, 81920)
            }
        })
        .collect()
}

fn bench_place_gpu_affinity(c: &mut Criterion) {
    let fleet = make_fleet(50);
    let refs: Vec<&NodeInfo> = fleet.iter().collect();
    let req = PlacementRequest {
        policy: PlacementPolicy::GpuAffinity,
        required_gpu: true,
        min_gpu_vram_mb: 40000,
        required_capabilities: Vec::new(),
        preferred_node: None,
        hardware: None,
    };

    c.bench_function("place GpuAffinity (50 nodes)", |b| {
        b.iter(|| place(&req, &refs));
    });
}

fn bench_place_balanced(c: &mut Criterion) {
    let fleet = make_fleet(50);
    let refs: Vec<&NodeInfo> = fleet.iter().collect();
    let req = PlacementRequest {
        policy: PlacementPolicy::Balanced,
        required_gpu: false,
        min_gpu_vram_mb: 0,
        required_capabilities: Vec::new(),
        preferred_node: None,
        hardware: None,
    };

    c.bench_function("place Balanced (50 nodes)", |b| {
        b.iter(|| place(&req, &refs));
    });
}

fn bench_place_locality(c: &mut Criterion) {
    let fleet: Vec<NodeInfo> = (0..50)
        .map(|i| {
            let caps: Vec<String> = ["python", "docker", "cuda", "rust"]
                .iter()
                .take((i % 4) + 1)
                .map(|s| s.to_string())
                .collect();
            cpu_node(&format!("node-{i}")).with_capabilities(caps)
        })
        .collect();
    let refs: Vec<&NodeInfo> = fleet.iter().collect();
    let req = PlacementRequest {
        policy: PlacementPolicy::Locality,
        required_gpu: false,
        min_gpu_vram_mb: 0,
        required_capabilities: vec!["python".into(), "docker".into(), "cuda".into()],
        preferred_node: None,
        hardware: None,
    };

    c.bench_function("place Locality (50 nodes, 3 caps)", |b| {
        b.iter(|| place(&req, &refs));
    });
}

fn bench_place_with_hardware_req(c: &mut Criterion) {
    let fleet = make_fleet(50);
    let refs: Vec<&NodeInfo> = fleet.iter().collect();
    let req = PlacementRequest {
        policy: PlacementPolicy::Balanced,
        required_gpu: false,
        min_gpu_vram_mb: 0,
        required_capabilities: Vec::new(),
        preferred_node: None,
        hardware: Some(HardwareRequirement {
            accelerators: vec![AcceleratorType::Cuda],
            min_memory_mb: 40000,
            min_device_count: 2,
            min_cpu_cores: 32,
            required_family: None,
        }),
    };

    c.bench_function("place HW requirement (50 nodes)", |b| {
        b.iter(|| place(&req, &refs));
    });
}

fn bench_rank_nodes_large(c: &mut Criterion) {
    let fleet = make_fleet(200);
    let refs: Vec<&NodeInfo> = fleet.iter().collect();
    let req = PlacementRequest {
        policy: PlacementPolicy::GpuAffinity,
        required_gpu: true,
        min_gpu_vram_mb: 0,
        required_capabilities: Vec::new(),
        preferred_node: None,
        hardware: None,
    };

    c.bench_function("rank_nodes GpuAffinity (200 nodes)", |b| {
        b.iter(|| rank_nodes(&req, &refs));
    });
}

fn bench_rank_nodes_cost(c: &mut Criterion) {
    let fleet = make_fleet(200);
    let refs: Vec<&NodeInfo> = fleet.iter().collect();
    let req = PlacementRequest {
        policy: PlacementPolicy::Cost,
        required_gpu: false,
        min_gpu_vram_mb: 0,
        required_capabilities: Vec::new(),
        preferred_node: None,
        hardware: None,
    };

    c.bench_function("rank_nodes Cost (200 nodes)", |b| {
        b.iter(|| rank_nodes(&req, &refs));
    });
}

criterion_group!(
    benches,
    bench_place_gpu_affinity,
    bench_place_balanced,
    bench_place_locality,
    bench_place_with_hardware_req,
    bench_rank_nodes_large,
    bench_rank_nodes_cost,
);
criterion_main!(benches);
