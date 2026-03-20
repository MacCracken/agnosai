# Building Community WASM Tools

AgnosAI supports community-authored tools compiled to WebAssembly. WASM tools run inside a sandboxed environment with memory isolation, CPU limits, and no filesystem or network access.

## Prerequisites

```bash
rustup target add wasm32-wasip1
```

## Quick Start

### 1. Create a new Rust project

```bash
cargo new my-tool
cd my-tool
```

### 2. Add the SDK dependency

```toml
[package]
name = "my-tool"
version = "1.0.0"
edition = "2024"

[dependencies]
agnosai-tool-sdk = { git = "https://github.com/maccracken/agnosai", path = "sdk/agnosai-tool-sdk" }
serde_json = "1"
```

### 3. Implement the tool

```rust
use agnosai_tool_sdk::{run_tool, ToolInput, ToolResult};

fn execute(input: ToolInput) -> ToolResult {
    let name = input.get_str("name").unwrap_or("world");
    ToolResult::ok(serde_json::json!({
        "greeting": format!("Hello, {name}!"),
    }))
}

fn main() {
    run_tool(execute);
}
```

### 4. Build for WASM

```bash
cargo build --target wasm32-wasip1 --release
```

The output is at `target/wasm32-wasip1/release/my-tool.wasm`.

### 5. Write a manifest

Create `manifest.json`:

```json
{
  "name": "my_tool",
  "description": "A custom greeting tool",
  "version": "1.0.0",
  "parameters": [
    {
      "name": "name",
      "description": "Who to greet",
      "param_type": "string",
      "required": false
    }
  ]
}
```

### 6. Package for deployment

Place both files in a directory:

```
my_tool/
├── manifest.json
└── my_tool.wasm
```

## Protocol

WASM tools communicate via stdin/stdout JSON:

**Input** (stdin):
```json
{"parameters": {"name": "Alice", "count": 42}}
```

**Output** (stdout, success):
```json
{"success": true, "result": {"greeting": "Hello, Alice!"}}
```

**Output** (stdout, failure):
```json
{"success": false, "error": "missing required parameter: name"}
```

## SDK API

| Type | Description |
|------|-------------|
| `ToolInput` | Parameter map with `get_str()`, `get_f64()`, `get_u64()`, `get_bool()` helpers |
| `ToolResult` | `ToolResult::ok(value)` or `ToolResult::err(msg)` |
| `run_tool(fn)` | Main entry point — reads stdin, calls handler, writes stdout |
| `ToolManifest` | Serializable manifest for package metadata |
| `ParameterSchema` | Parameter definition (name, description, type, required) |

## Sandbox Limits

| Resource | Default Limit |
|----------|--------------|
| Memory | 64 MiB |
| CPU (fuel) | ~1 billion instructions |
| Timeout | 30 seconds |
| Filesystem | None |
| Network | None |
| Environment | None |

## Loading Tools in AgnosAI

Tools are loaded from a directory of packages:

```rust
use agnosai::tools::wasm_loader::load_all_tool_packages;
use agnosai::sandbox::wasm::WasmSandbox;

let sandbox = Arc::new(WasmSandbox::new()?);
let count = load_all_tool_packages(Path::new("./tools"), &sandbox, &registry)?;
```

Or load a single package:

```rust
use agnosai::tools::wasm_loader::load_tool_package;

let tool = load_tool_package(Path::new("./tools/my_tool"), &sandbox)?;
registry.register(Arc::new(tool));
```

## Example

See `examples/wasm-tools/hello-tool/` for a complete working example.
