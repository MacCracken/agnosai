# AgnosAI

Rust-native agent orchestration engine. Multi-agent crews with task DAGs, LLM routing, fleet distribution, and sandboxed tool execution.

AgnosAI replaces Python/CrewAI orchestration with a compiled Rust binary — real concurrency, zero GIL, predictable performance. Use it standalone or as the core engine inside [Agnostic](https://github.com/maccracken/agnostic).

## Why

| Problem with Python/CrewAI | AgnosAI |
|---|---|
| GIL serializes concurrent crews | Real threads via tokio |
| 200+ transitive dependencies | ~30 curated Rust crates |
| 1.5 GB container image | <50 MB static binary |
| 15-30s boot time | <2s to agent-ready |
| No fleet awareness | Native multi-node distribution |
| Unsandboxed tool execution | WASM / seccomp / Landlock / OCI |
| Sequential or hierarchical only | Arbitrary task DAGs with priority + preemption |

## Architecture

```
agnosai (workspace)
├── agnosai-core          Core types, traits, error handling
├── agnosai-orchestrator  Task scheduling, agent coordination, pub/sub
├── agnosai-llm           LLM provider abstraction (9 providers, native HTTP)
├── agnosai-fleet         Distributed fleet coordination, GPU scheduling
├── agnosai-sandbox       Tool execution isolation (WASM, process, OCI)
├── agnosai-tools         Tool registry & execution (native, WASM, Python bridge)
├── agnosai-learning      Adaptive learning & reinforcement learning
├── agnosai-server        HTTP/gRPC API server (REST, MCP, A2A, SSE)
└── agnosai-definitions   Preset library, crew assembly, packaging
```

## Quick Start

```bash
# Build everything
cargo build

# Run with a simple crew definition
cargo run --example simple_crew

# Run the API server
cargo run -p agnosai-server

# Run tests
cargo test
```

## Usage as a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
agnosai-core = { git = "https://github.com/maccracken/agnosai" }
agnosai-orchestrator = { git = "https://github.com/maccracken/agnosai" }
```

```rust
use agnosai_core::{AgentDefinition, Task, CrewSpec};
use agnosai_orchestrator::Orchestrator;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let orchestrator = Orchestrator::new(Default::default()).await?;

    let crew = CrewSpec::builder()
        .name("analysis-crew")
        .agent(AgentDefinition::from_file("agents/analyst.json")?)
        .agent(AgentDefinition::from_file("agents/reviewer.json")?)
        .task(Task::new("Analyze the codebase for security issues"))
        .build();

    let result = orchestrator.run_crew(crew).await?;
    println!("{}", result.summary());
    Ok(())
}
```

## Agent Definitions

Agents are defined declaratively in JSON or YAML — same format as Agnostic v1 presets:

```json
{
  "agent_key": "senior-qa-engineer",
  "name": "Senior QA Engineer",
  "role": "Senior QA Engineer",
  "goal": "Ensure comprehensive test coverage and quality standards",
  "domain": "quality",
  "tools": ["code_analysis", "test_generation", "edge_case_analysis"],
  "complexity": "high",
  "llm_model": "capable"
}
```

## LLM Providers

Native HTTP implementations — no Python SDKs, no litellm:

| Provider | Protocol |
|---|---|
| OpenAI | REST (`/v1/chat/completions`) |
| Anthropic | REST (`/v1/messages`) |
| Google Gemini | REST (`/v1beta/models`) |
| Ollama | REST (`/api/chat`) |
| DeepSeek | OpenAI-compatible |
| Mistral | OpenAI-compatible |
| Groq | OpenAI-compatible |
| LM Studio | OpenAI-compatible |
| AGNOS hoosh | OpenAI-compatible gateway |

Task-complexity routing automatically selects the right model tier (Fast / Capable / Premium).

## Tool Execution

Tools run in three tiers with increasing isolation:

1. **Native Rust** — in-process, zero overhead
2. **WASM** — wasmtime sandbox, memory-isolated, capability-controlled
3. **Sandboxed Python** — subprocess with seccomp-bpf + Landlock + cgroups + network namespace

## Fleet Distribution

First-class multi-node support:

- Node registry with heartbeat + TTL
- 5 placement policies (GPU-affinity, balanced, locality, cost, manual)
- Inter-node relay via Redis pub/sub or gRPC
- Barrier sync and checkpoint-based crew state
- GPU detection and VRAM-aware scheduling
- Multi-cluster federation

## Project Status

See [docs/development/roadmap.md](docs/development/roadmap.md) for the full development plan and current phase.

## License

Apache-2.0
