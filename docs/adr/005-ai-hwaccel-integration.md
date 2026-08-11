# ADR-005: ai-hwaccel for Hardware Detection and Workload Planning

## Status: Accepted — Phase 1 shipped, Phase 2 declined

**Date**: 2026-03-19 (Rust era). **Amended 2026-08-11.** The decision to
integrate ai-hwaccel holds and is fully realised. Two things below were wrong or
have changed: the variant count (13 → **19**, and it was already 19 before the
port), and the two-phase migration path — **Phase 2 never happened, in either
tree**, and the port's shape says it was rejected rather than deferred.

## Context

AgnosAI needs to detect available hardware (GPU, TPU, NPU) and route workloads to the right compute. We initially implemented a minimal `AcceleratorType` enum (6 variants) and `HardwareInventory` with manual device registration.

The sibling AGNOS project `ai-hwaccel` provides a comprehensive hardware detection library with automatic detection via sysfs/CLI probing, quantization planning, model sharding, and training memory estimation.

## Decision

Integrate `ai-hwaccel` for detection and planning, and **keep agnosai's own
6-variant `AcceleratorType` as the placement-facing type**, narrowing
ai-hwaccel's richer enum at the boundary.

## Integration Surface — as built

| ai-hwaccel API | Cyrius symbol | AgnosAI usage | Where |
|---|---|---|---|
| `AcceleratorRegistry::detect()` | `registry_detect()` | `agnosai_hw_inventory_detect` | `src/core/resource.cyr` |
| `AcceleratorProfile` | `profile_accel_type` / `profile_memory_bytes` / `profile_mem_free` | `agnosai_hw_inventory_from_hwaccel` | `src/core/resource.cyr` |
| `suggest_quantization(params)` | `reg_suggest_quant(registry, params)` | `agnosai_suggest_quantization` | `src/llm/router.cyr` |
| `AcceleratorRegistry::estimate_memory` | `reg_estimate_memory(params, quant)` | `agnosai_estimate_model_memory` | `src/llm/router.cyr` |
| `plan_sharding(params, quant)` | `reg_plan_sharding(registry, params, quant)` | `agnosai_fleet_plan_sharding` | `src/fleet/coordinator.cyr` |
| `estimate_training_memory(params, method, target)` | same name | `agnosai_training_memory_estimate` | `src/core/resource.cyr` |

⚠ **`suggest_quantization`'s argument order is flipped.** ai-hwaccel's
`reg_suggest_quant` takes the registry first, as every other `reg_*` fn does.
Following the oracle's `suggest_quantization(model_params, registry)` order would
still *compile* — Cyrius has no types on these parameters — and would then walk a
model's parameter count as if it were a registry struct.

⚠ `estimate_training_memory` takes **three** arguments (params, method, target),
not the two this ADR originally listed.

⚠ **`AcceleratorRegistry::satisfying(req)` for "filter nodes in placement
engine" was never implemented**, in `rust-old/` or in `src/`. ai-hwaccel has
`find_satisfying_profile` / `count_satisfying`; nothing in agnosai calls either.
`src/fleet/placement.cyr` filters on agnosai's own `ComputeDevice` fields. The
row was aspirational when written and is left here marked as such rather than
deleted, because a future reader may reasonably want to close the gap.

## Migration Path — what actually happened

### Phase 1: Optional (Rust)

Realised as designed. `rust-old/Cargo.toml` declares
`ai-hwaccel = { version = "1.0.0", optional = true }` with
`hwaccel = ["dep:ai-hwaccel"]`, and `HardwareInventory::detect` and
`AcceleratorType::from_hwaccel` sit behind `#[cfg(feature = "hwaccel")]`. (The
ADR's snippet showed a `path = "../ai-hwaccel"` dependency; the shipped manifest
uses a version, not a path.)

### Phase 1 in Cyrius: not optional at all

**The port has no feature gate.** `[deps.ai-hwaccel]` is declared unconditionally
in `cyrius.cyml` (currently tag **2.3.16**), and every `#[cfg(feature =
"hwaccel")]` function in the oracle is ported and built:
`agnosai_accelerator_type_from_hwaccel`, `agnosai_hw_inventory_from_hwaccel`,
`agnosai_hw_inventory_detect`, `agnosai_training_memory_estimate` and its four
accessors (`src/core/resource.cyr`), `agnosai_suggest_quantization` and
`agnosai_estimate_model_memory` (`src/llm/router.cyr`), and
`agnosai_fleet_plan_sharding` (`src/fleet/coordinator.cyr`). A cargo feature gate
is not a scope boundary.

So "no impact on builds that don't need hardware detection" no longer applies —
there is one build, and it carries ai-hwaccel.

### Phase 2: Required — **declined, not pending**

Phase 2 proposed replacing `agnosai_core::AcceleratorType` with a re-export of
ai-hwaccel's enum and `HardwareInventory` with a thin wrapper over
`AcceleratorRegistry`. Neither happened in Rust, and the port did the opposite
deliberately:

- **agnosai's 6-variant `AgnosaiAcceleratorType` is still the placement type**
  (`src/core/resource.cyr`), unchanged: CPU, CUDA, ROCM, METAL, VULKAN, TPU.
- **`HardwareInventory` is still agnosai's own struct**, built *from* a registry
  by `agnosai_hw_inventory_from_hwaccel` rather than wrapping one.
- The bridge is a **lossy narrowing**, and the oracle chose that fallback on
  purpose: anything without a direct mapping (NPUs, ASICs) becomes **CPU**, so
  the device stays usable for CPU work but cannot match a GPU or TPU placement
  requirement it could not serve. `rust-old/src/core/resource.rs:82-97` says so;
  `agnosai_accelerator_type_from_hwaccel` reproduces it.

⚠ **The mapper is not a pass-through, and it looks like one.** ai-hwaccel numbers
`ACCEL_TPU = 8` while `AGNOSAI_ACCEL_TPU = 5`. CPU/CUDA/ROCM/METAL/VULKAN are 0–4
on both sides and would survive a bare integer cast — five of six values right by
coincidence, and a TPU silently becoming an Intel NPU. The explicit ladder is
load-bearing.

The oracle's `pub use ai_hwaccel::{…}` re-export block (`HwAccelType`,
`QuantizationLevel`, `ShardingPlan`, `TrainingMethod`, …) has **no counterpart
and needs none**: Cyrius has one flat namespace, so `QUANT_FP16`, `TRAIN_LORA`,
`est_total_x1000` and the rest are already visible to every consumer.
Re-exporting would mean defining duplicate symbols, which is the one thing a flat
namespace punishes.

## Rationale

- ai-hwaccel detects far more hardware types than agnosai's six — TPU, Gaudi,
  Neuron, and a range of NPUs
- Detection is automatic (sysfs + CLI) vs manual registration
- Quantization and sharding planning are production-ready, and a second
  implementation in agnosai would drift from them silently
- Same AGNOS ecosystem — aligned versioning and design philosophy
- Zero compile-time SDK dependencies

⚠ **The "13 accelerator types vs our 6" figure is stale and was already stale
before the port.** `lib/ai-hwaccel.cyr` (2.3.16) declares **19** variants —
`ACCEL_CPU` (0) through `ACCEL_WIN_GPU` (18) — and
`rust-old/src/core/resource.rs:38` already described a "19-variant enum" at the
v1.1.0 freeze. Do not quote a count from this ADR; read the enum.

## Consequences

- **ai-hwaccel is a hard dependency of the Cyrius build.** The version needs
  keeping in step with agnosai releases, and a variant added upstream lands in
  the `_` fallback (→ CPU) until the mapper is extended
- `AgnosaiAcceleratorType` staying at 6 variants is now a **settled** choice, not
  a temporary shim. Consumers that need the full fidelity read ai-hwaccel's
  profile directly rather than going through the inventory
- `agnosai_hw_inventory_detect` shells out to `nvidia-smi` and friends and is
  therefore untested — as in the oracle, which has no test for `detect()` either.
  `from_hwaccel` is the testable half and is where the assertions go
- Two inventory behaviours are easy to get wrong and are documented at the call
  site: the **CPU profile is not a device** (it feeds `memory_total_mb` and
  `cpu_cores` and is skipped), and device `index` is the position in the
  registry's profile list, so a CPU at position 0 leaves a gap
