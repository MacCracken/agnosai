# Ifran Integration Plan

> Integration of [ifran](https://github.com/MacCracken/ifran) LLM training capabilities into AgnosAI.

## Status

**Pending** — ifran is in active development (workspace layout, not yet flattened to a single crate on crates.io).

## What ifran Provides

ifran is a Rust LLM controller covering model management, inference, and training:

- **Training methods**: LoRA, QLoRA, full fine-tune, DPO, RLHF, distillation
- **Distributed training**: multi-node coordination with checkpointing
- **Experiments**: autonomous hyperparameter sweeps (grid/random/Bayesian)
- **Evaluation**: MMLU, HellaSwag, HumanEval, perplexity benchmarks
- **Dataset management**: loading, splitting, preprocessing
- **Checkpointing**: save/resume training state

## Integration Points

### 1. Agent Fine-Tuning from Crew Feedback

AgnosAI's `learning` module (UCB1, Q-learning, capability scoring) produces performance data per agent. ifran-train could use this to:

- Fine-tune agent LLMs based on successful crew runs (LoRA/QLoRA)
- Use DPO with crew approval gate decisions as preference pairs
- Run evaluation benchmarks on fine-tuned models before deployment

### 2. Training Orchestration via Fleet

AgnosAI's fleet module manages distributed GPU resources. ifran-train's distributed training could use:

- `fleet::gpu::ComputeScheduler` for VRAM allocation
- `fleet::topology` for NVLink/XGMI-aware training placement
- `fleet::cost_planning` for training cost estimation

### 3. Model Lifecycle

ifran manages model pulling, versioning, and deployment. AgnosAI's `llm::router` could query ifran for:

- Available models and their capabilities
- Quantization recommendations from `ai-hwaccel`
- Model performance benchmarks for routing decisions

## Dependency Plan

When ifran is published as a flat crate:

```toml
[dependencies]
ifran = { version = "1.0", default-features = false, features = ["train"], optional = true }

[features]
training = ["dep:ifran"]
```

## Current Workaround

AgnosAI's learning module provides runtime RL (online learning during crew execution). For offline model training, use ifran directly via its CLI or API — the two systems complement each other without tight coupling.
