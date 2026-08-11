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
├── benches/              Benchmarks — flat *.bcyr, discovery is NOT recursive
├── tests/                Test suites — *.tcyr, discovered recursively
├── examples/             Runnable examples — *.cyr, built by CI
├── rust-old/             The frozen Rust v1.1.0 tree, kept as the parity oracle
└── docs/                 Guides, ADRs, architecture docs
```

The `[feature: …]` tags above are the Rust tree's cargo features. **They are not
scope boundaries here** — every one of those modules is ported and built
unconditionally.

See [Architecture Overview](docs/architecture/overview.md) for detailed design.

## Quick Start

```bash
# Resolve dependencies into lib/
cyrius deps

# Build — produces build/agnosai
cyrius build src/main.cyr build/agnosai

# Run the API server
./build/agnosai

# Run every test suite (recursive)
cyrius tests tests

# Benchmarks, and the 80% coverage gate
cyrius bench
cyrius coverage --min 80

# Everything CI runs, locally
./scripts/check-clean.sh && ./scripts/check-symbols.sh
```

> **The binary is `agnosai`, not `agnosai-server`.** The Rust tree built a
> `[[bin]] agnosai-server`; the Cyrius tree builds one binary named for the
> project, declared in `cyrius.cyml` as `[build].output` and `[release].bins`.
> There is no `Cargo.toml` and no `make check`.

## Usage as a Library

Add to your `cyrius.cyml`:

```toml
[deps.agnosai]
git = "https://github.com/MacCracken/agnosai.git"
tag = "2.0.0"
```

Then `include` the groups you need — Cyrius resolution is single-pass, so callees
must precede callers. A complete, runnable version of the example below is
[`examples/simple_crew.cyr`](examples/simple_crew.cyr):

```cyr
var orchestrator = agnosai_orchestrator_new(0);

var crew = agnosai_crew_new(str_from("example-crew"));
var tasks = vec_new();
vec_push(tasks, agnosai_task_new(str_from("Analyze the project structure")));
agnosai_crew_with_tasks(crew, tasks);

var state = agnosai_orchestrator_run_crew(orchestrator, crew);
# -> "completed", with each task's output in agnosai_crew_state_results(state)
```

<details>
<summary>The equivalent in the frozen Rust tree, for comparison</summary>

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

</details>

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
$ cyrius tests tests
...
99 passed, 0 failed
```

```
$ cyrius coverage --min 80
Functions referenced: 1561/1561 (100%)  [reference coverage — a floor, not a correctness proof]
```

Tests cover core types, orchestration (all 4 process modes), DAG cycle detection,
agent scoring, priority scheduling, pub/sub, IPC, LLM provider routing, the tool
registry, API routes, the WASM and process sandboxes, and a deterministic parser
fuzz sweep.

The parity bar is the frozen Rust tree at [`rust-old/`](rust-old/), which carries
**863** test functions across 84 modules; the Cyrius suites are organised
differently and go past it in several places — see
[`docs/development/roadmap.md`](docs/development/roadmap.md).

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

GPL-3.0-only — see [LICENSE](LICENSE) for details.
