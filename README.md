# AgnosAI

Rust-native agent orchestration engine. Multi-agent crews with task DAGs, LLM routing, fleet distribution, and sandboxed tool execution.

AgnosAI replaces Python/CrewAI orchestration with a compiled Rust binary -- real concurrency, zero GIL, predictable performance. Use it standalone or as the core engine inside [Agnostic](https://github.com/maccracken/agnostic).

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
agnosai
├── src/
│   ├── core/             Core types, traits, error handling
│   ├── orchestrator/     Task scheduling, agent scoring, crew execution
│   ├── llm/              LLM provider abstraction (8 providers, native HTTP)
│   ├── fleet/            Distributed fleet coordination, GPU scheduling [feature: fleet]
│   ├── sandbox/          Tool execution isolation (WASM, process, OCI) [feature: sandbox]
│   ├── tools/            Tool registry & execution (native, WASM, Python bridge)
│   ├── learning/         Adaptive learning & reinforcement learning
│   ├── server/           HTTP API server (REST, health probes, SSE)
│   └── definitions/      Preset library, crew assembly, packaging [feature: definitions]
├── benches/              Criterion benchmarks
├── tests/                Integration tests
├── examples/             Usage examples
└── docs/                 Guides, ADRs, architecture docs
```

See [Architecture Overview](docs/architecture/overview.md) for detailed design.

## Quick Start

```bash
# Build
cargo build

# Run the API server
cargo run --bin agnosai-server

# Run tests
cargo test

# Run all CI checks locally
make check
```

## Usage as a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
agnosai = { git = "https://github.com/maccracken/agnosai" }
```

```rust
use agnosai::core::{AgentDefinition, CrewSpec, Task, ProcessMode, TaskPriority};
use agnosai::orchestrator::Orchestrator;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let orchestrator = Orchestrator::new(Default::default()).await?;

    // Define agents
    let analyst = AgentDefinition::from_json(r#"{
        "agent_key": "analyst",
        "name": "Analyst",
        "role": "data analyst",
        "goal": "analyze data",
        "domain": "data-engineering",
        "tools": ["json_transform"],
        "complexity": "high"
    }"#)?;

    // Build a crew with tasks and dependencies
    let mut task_a = Task::new("Gather quarterly revenue data");
    task_a.priority = TaskPriority::High;
    let mut task_b = Task::new("Analyze trends and anomalies");
    task_b.dependencies.push(task_a.id);

    let mut crew = CrewSpec::new("analysis-crew");
    crew.agents = vec![analyst];
    crew.tasks = vec![task_a, task_b];
    crew.process = ProcessMode::Dag;

    let result = orchestrator.run_crew(crew).await?;
    for r in &result.results {
        println!("[{}] {}", r.status, r.output);
    }
    Ok(())
}
```

## Agent Definitions

Agents are defined declaratively in JSON -- same format as Agnostic v1 presets:

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

Native HTTP implementations -- no Python SDKs, no litellm:

| Provider | Protocol |
|---|---|
| OpenAI | REST (`/v1/chat/completions`) |
| Anthropic | REST (`/v1/messages`) |
| Ollama | REST (`/api/chat`) |
| DeepSeek | OpenAI-compatible |
| Mistral | OpenAI-compatible |
| Groq | OpenAI-compatible |
| LM Studio | OpenAI-compatible |
| AGNOS hoosh | OpenAI-compatible gateway |

Task-complexity routing automatically selects the right model tier (Fast / Capable / Premium).

## Tool Execution

Tools run in three tiers with increasing isolation:

1. **Native Rust** -- in-process, zero overhead
2. **WASM** -- wasmtime sandbox, memory-isolated, capability-controlled
3. **Sandboxed Python** -- subprocess with seccomp-bpf + Landlock + cgroups + network namespace

## Fleet Distribution

First-class multi-node support:

- Node registry with heartbeat + TTL
- 5 placement policies (GPU-affinity, balanced, locality, cost, manual)
- Inter-node relay via Redis pub/sub or gRPC
- Barrier sync and checkpoint-based crew state
- GPU detection and VRAM-aware scheduling
- Multi-cluster federation

## Test Suite

```
$ cargo test
...
test result: ok. 309 passed; 0 failed; 0 ignored
```

Tests cover core types, orchestration (all 4 process modes), DAG cycle detection, agent scoring, priority scheduling, pub/sub, IPC, LLM provider routing, tool registry, and API routes.

## Documentation

See the [docs/](docs/index.md) directory:

- [Getting Started](docs/guides/getting-started.md)
- [Architecture Overview](docs/architecture/overview.md)
- [Crew Execution Patterns](docs/guides/crew-patterns.md)
- [API Reference](docs/guides/api-reference.md)
- [Adding LLM Providers](docs/guides/adding-providers.md)
- [Adding Native Tools](docs/guides/adding-tools.md)
- [Roadmap](docs/development/roadmap.md)

## Project Status

See [docs/development/roadmap.md](docs/development/roadmap.md) for the full development plan and current phase.

## License

AGPL-3.0-only — see [LICENSE](LICENSE) for details.
