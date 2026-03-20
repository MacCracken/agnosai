# ADR-005: ai-hwaccel for Hardware Detection and Workload Planning

**Status:** Accepted
**Date:** 2026-03-19

## Context

AgnosAI needs to detect available hardware (GPU, TPU, NPU) and route workloads to the right compute. We initially implemented a minimal `AcceleratorType` enum (6 variants) and `HardwareInventory` with manual device registration.

The sibling AGNOS project `ai-hwaccel` provides a comprehensive hardware detection library with 13 accelerator types, automatic detection via sysfs/CLI probing, quantization planning, model sharding, and training memory estimation.

## Decision

Integrate `ai-hwaccel` as:

1. **Optional now** — feature-gated dependency. AgnosAI works without it (manual hardware registration). When enabled, auto-detects hardware and populates inventory from real system probes.

2. **Required future** — replace AgnosAI's hand-rolled hardware types with `ai-hwaccel`'s richer model. The 6-variant `AcceleratorType` becomes a compatibility shim over `ai-hwaccel`'s 13-variant enum.

## Integration Surface

| ai-hwaccel API | AgnosAI Usage |
|---|---|
| `AcceleratorRegistry::detect()` | Auto-populate `HardwareInventory` at node startup |
| `AcceleratorRegistry::satisfying(req)` | Filter nodes in placement engine |
| `suggest_quantization(params)` | LLM provider model selection |
| `plan_sharding(params, quant)` | Fleet coordinator workload distribution |
| `estimate_training_memory(params, method)` | Resource budget validation |
| `AcceleratorProfile` | Richer device info for scoring |

## Migration Path

### Phase 1: Optional (current)
```toml
[dependencies]
ai-hwaccel = { path = "../ai-hwaccel", optional = true }

[features]
hwaccel = ["dep:ai-hwaccel"]
```

- `HardwareInventory::detect()` method behind `#[cfg(feature = "hwaccel")]`
- Falls back to manual registration when feature is off
- Map `ai-hwaccel::AcceleratorType` → `agnosai_core::AcceleratorType`

### Phase 2: Required
- Replace `agnosai_core::AcceleratorType` with re-export from `ai-hwaccel`
- Replace `HardwareInventory` with thin wrapper around `AcceleratorRegistry`
- Use `plan_sharding()` in fleet coordinator
- Use `suggest_quantization()` in LLM model router

## Rationale

- ai-hwaccel detects 13 hardware types vs our 6 — covers TPU, Gaudi, Neuron, NPUs
- Detection is automatic (sysfs + CLI) vs manual registration
- Quantization and sharding planning are production-ready
- Same AGNOS ecosystem — aligned versioning and design philosophy
- Zero compile-time SDK dependencies

## Consequences

- Optional feature flag means no impact on builds that don't need hardware detection
- When required, `ai-hwaccel` becomes a core dependency (~like `serde`)
- `AcceleratorType` enum will eventually change (13 variants, `#[non_exhaustive]`)
- Need to keep `ai-hwaccel` version-aligned with AgnosAI releases
