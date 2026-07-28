# Getting Started

## Prerequisites

- Rust 1.89+ (`rustup update`)
- For AGNOS tools: running instances of Synapse, Mneme, or Delta (optional)

## Build

```bash
git clone https://github.com/maccracken/agnosai.git
cd agnosai
cargo build
```

## Run the Example

```bash
cargo run --example simple_crew
```

This creates a crew with a single task and executes it. Output:

```
Crew completed with status: Completed
```

## Run the API Server

```bash
cargo run --bin agnosai-server
```

The server starts on `http://localhost:8080` with:
- `GET /health` — liveness probe
- `GET /ready` — readiness probe

## Run Tests

```bash
cargo test
```

## Using as a Library

Add to your project's `Cargo.toml`:

```toml
[dependencies]
agnosai = { git = "https://github.com/maccracken/agnosai" }
```

### Minimal Crew

```rust
use agnosai::core::{CrewSpec, Task};
use agnosai::orchestrator::Orchestrator;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let orchestrator = Orchestrator::new(Default::default()).await?;

    let mut crew = CrewSpec::new("my-crew");
    crew.tasks.push(Task::new("Analyze the codebase"));

    let result = orchestrator.run_crew(crew).await?;
    println!("Status: {:?}", result.status);
    Ok(())
}
```

### With Agent Definitions

```rust
use agnosai::definitions::loader;

// Load from JSON file
let agent = loader::load_from_file(Path::new("agents/analyst.json"))?;

// Or from a directory
let agents = loader::load_all_from_dir(Path::new("agents/"))?;

// Build a crew with loaded agents
let mut crew = CrewSpec::new("analysis-crew");
crew.agents = agents;
crew.tasks.push(Task::new("Find security vulnerabilities"));
```

### DAG Execution

```rust
use agnosai::core::task::ProcessMode;

let mut task_a = Task::new("Gather requirements");
let mut task_b = Task::new("Write code");
let mut task_c = Task::new("Review code");

// B depends on A, C depends on B
task_b.dependencies.push(task_a.id);
task_c.dependencies.push(task_b.id);

let mut crew = CrewSpec::new("dev-crew");
crew.process = ProcessMode::Dag;
crew.tasks = vec![task_a, task_b, task_c];
```

### Using Tools

```rust
use agnosai::tools::{ToolRegistry, NativeTool};
use agnosai::tools::builtin::echo::EchoTool;
use std::sync::Arc;

let registry = ToolRegistry::new();
registry.register(Arc::new(EchoTool));

// List available tools
for schema in registry.list() {
    println!("{}: {}", schema.name, schema.description);
}
```

### AGNOS Ecosystem Tools

```rust
use agnosai::tools::builtin::synapse::SynapseInfer;
use agnosai::tools::builtin::mneme::MnemeSearch;
use agnosai::tools::builtin::delta::DeltaListRepos;

// Default URLs (localhost)
let synapse = Arc::new(SynapseInfer::new());
let mneme = Arc::new(MnemeSearch::new());
let delta = Arc::new(DeltaListRepos::new());

// Custom URLs
let synapse = Arc::new(SynapseInfer::with_base_url("http://synapse.internal:8420".into()));

registry.register(synapse);
registry.register(mneme);
registry.register(delta);
```
