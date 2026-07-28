//! Example AgnosAI WASM tool — greets the user.
//!
//! Build with:
//!   rustup target add wasm32-wasip1
//!   cargo build --target wasm32-wasip1 --release
//!
//! The output .wasm lives at:
//!   target/wasm32-wasip1/release/hello-tool.wasm

use agnosai_tool_sdk::{run_tool, ToolInput, ToolResult};

fn execute(input: ToolInput) -> ToolResult {
    let name = input.get_str("name").unwrap_or("world");
    let greeting = format!("Hello, {name}!");

    ToolResult::ok(serde_json::json!({
        "greeting": greeting,
        "name": name,
    }))
}

fn main() {
    run_tool(execute);
}
