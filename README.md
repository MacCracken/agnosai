# AgnosAI

Cyrius-native agent orchestration engine. Multi-agent crews with task DAGs, LLM routing, fleet distribution, and sandboxed tool execution.

AgnosAI replaces Python/CrewAI orchestration with a compiled Cyrius binary -- real OS threads, no GIL, predictable performance. Use it standalone or as the core engine inside [Agnostic](https://github.com/maccracken/agnostic).

> **This is a Cyrius project.** It began as Rust; that tree is frozen at
> `rust-old/` as history, not a spec, and is **scheduled for deletion** — the
> 2.1.0 audit found nothing that still depends on it. The 2.1.0 notes put the
> deletion in the next release; 2.1.1 to 2.1.6 did not carry it, so it is still
> open on the roadmap. Nothing under `rust-old/` is built or shipped.

## Why

| Problem with Python/CrewAI | AgnosAI |
|---|---|
| GIL serializes concurrent crews | Real OS threads via the sandhi worker pool |
| 200+ transitive dependencies | 6 curated Cyrius packages + a pinned stdlib snapshot |
| 1.5 GB container image | <50 MB static binary |
| 15-30s boot time | <2s to agent-ready |
| No fleet awareness | Native multi-node distribution |
| Unsandboxed tool execution | WASM (fuel + memory bounded) or OCI |
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
├── rust-old/             The frozen Rust v1.1.0 tree — scheduled for deletion
└── docs/                 Guides, ADRs, architecture docs
```

The `[feature: …]` tags above are the Rust tree's cargo features. **They are not
scope boundaries here** — every one of those modules is ported and built
unconditionally.

See [Architecture Overview](docs/architecture/overview.md) for detailed design.

## Quick Start

```bash
# Provision the pinned stdlib snapshot into lib/ FIRST — `cyrius deps` only
# overlays [deps.NAME] on top of it, and fails against an empty lib/.
cyrius lib sync --full

# Resolve dependencies into lib/
cyrius deps

# Build — produces build/agnosai
cyrius build src/main.cyr build/agnosai

# Run the API server
./build/agnosai

# Run every test suite (recursive)
cyrius test tests

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

AgnosAI ships **both** a binary and a bundle. `cyrius.cyml` carries a `[lib]`
stanza of **111** modules, and `cyrius distlib` concatenates them into
`dist/agnosai.cyr` (**39,324 lines** at 2.1.6, stamped with the release) plus a
`dist/agnosai.deps` sidecar naming the **45** stdlib leaves the fold needs in
scope — so a `[deps.agnosai]` block resolves like any other dependency:

```cyml
[deps.agnosai]
git = "https://github.com/MacCracken/agnosai.git"
tag = "2.1.6"
modules = ["dist/agnosai.cyr"]
```

⚠ **`src/main.cyr` is deliberately absent from the bundle.** Its last two lines
are top-level statements that run at include time, so a consumer linking them
would start a server on include. Call the orchestrator directly
(`agnosai_orchestrator_submit_crew`) or stand up your own serve loop.

Vendoring the tree and `include`-ing the groups you need still works and is what
[`examples/simple_crew.cyr`](examples/simple_crew.cyr) does — 25 explicit
`include "src/…"` lines. Cyrius resolution is single-pass, so callees must
precede callers. A complete, runnable version of the example below is
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
  "tools": ["json_transform", "security_audit"],
  "complexity": "high"
}
```

An agent with no `llm_model` gets the model its `complexity` calls for, from what the gateway lists. To name one, use an
id from hoosh's `GET /v1/models/catalog`, exactly as listed: `"llm_model": "claude-sonnet-5-5"`.

## LLM Providers

⚠ **One HTTP path, not one per provider.** AgnosAI talks to the AGNOS **hoosh**
gateway over its OpenAI-compatible `/v1/chat/completions`, and hoosh owns the
per-provider protocols. There is no `/v1/messages` or `/api/chat` client in this
tree — a provider is a *label* that routes a model string, not a separate
transport.

agnosai carries no model names. Task complexity picks a tier (Fast / Capable / Premium), and what a tier means is read
from hoosh's catalog ([ADR 030](docs/adr/030-a-model-comes-from-the-gateways-catalog.md)): by hoosh's own tier for each
model on hoosh 2.10.2 or newer, and by what each model declares and costs on an older gateway. A model hoosh calls
anything but `chat` is never chosen, and an agent with tools is never given one hoosh says takes none. So a provider's
new model needs no agnosai release. `AGNOSAI_MODEL_FAST`, `AGNOSAI_MODEL_CAPABLE` and
`AGNOSAI_MODEL_PREMIUM` override a tier. ⚠ With no catalog and no override, a crew that names no model fails, with an
error starting `no model catalog:`, rather than send a guess.

## Tool Execution

An agent is offered the tools its `tools` list names, and the model calls them until it answers
([ADR 029](docs/adr/029-the-tool-loop.md)). A name the server does not hold fails the task, naming the tool. Every call
is journaled before it runs, a turn's calls are approved as a set, and what a tool writes reaches the model as data. Each
call is on the task's result under `tool_calls`, so what a tool measured can be told from what a model said.

Tools run in four tiers:

1. **Native** -- in-process, zero overhead. Review before registering.
2. **WASM** -- `wasmtime` with a **fuel budget** and a **memory ceiling**;
   WASI is stdio-only, so the guest gets no filesystem. kavach wraps the
   `wasmtime` process itself.
3. **OCI** -- a container: read-only root, no network unless the policy allows it,
   a memory limit, every capability dropped, `no-new-privileges` and at most 256
   processes.
4. **Process / Python** -- a subprocess under kavach's confinement since 2.3.4
   ([ADR 031](docs/adr/031-confine-the-process-and-python-tiers-in-agnosais-spawner.md)):
   NO_NEW_PRIVS, kavach's seccomp filter, and landlock rules that give read and
   execute on the system's own trees and read and write on `/tmp` and the call's
   work directory, and nothing else. Plus a sanitized environment and a SIGKILL
   deadline. ⚠ **The network is not isolated on this tier**, and a server running
   as root gives these tools root's capabilities, so the image runs as an
   unprivileged user. `AGNOSAI_SANDBOX_READ_PATHS` and `AGNOSAI_SANDBOX_WRITE_PATHS`
   give every tool another tree.

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
$ cyrius test tests
...
106 passed, 0 failed
```

```
$ cyrius coverage --min 80
Functions referenced: 1846/1857 (99%)  [reference coverage — a floor, not a correctness proof]
```

Tests cover core types, orchestration (all 4 process modes), DAG cycle detection,
agent scoring, priority scheduling, pub/sub, IPC, LLM provider routing, the tool
registry, API routes, the WASM and process sandboxes, and a deterministic parser
fuzz sweep.

The Rust tree this started from (`rust-old/`, scheduled for deletion) carried
**863** test functions across 84 modules; the 2.1.0 audit checked every one against
the Cyrius suites, which are organised differently and go past it in several places —
see [`docs/development/roadmap.md`](docs/development/roadmap.md).

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

See [docs/development/roadmap.md](docs/development/roadmap.md) for open work and [docs/development/state.md](docs/development/state.md) for where things stand.

## License

GPL-3.0-only — see [LICENSE](LICENSE) for details.
