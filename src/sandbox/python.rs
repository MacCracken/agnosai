//! Sandboxed Python interpreter for legacy tools.
//!
//! Protocol: stdin JSON -> python3 -> stdout JSON
//! Sandbox: seccomp + Landlock + cgroups + network namespace (future)

use std::path::PathBuf;
use std::time::Duration;

use crate::core::error::AgnosaiError;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// Result of executing a Python script in the sandbox.
#[derive(Debug)]
pub struct PythonResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub timed_out: bool,
}

/// Sandboxed Python subprocess bridge.
///
/// Spawns `python3 -c <script>` as a child process, pipes JSON on stdin,
/// captures stdout/stderr, and enforces a timeout.
pub struct PythonSandbox {
    python_path: PathBuf,
    timeout: Duration,
    work_dir: Option<PathBuf>,
}

impl PythonSandbox {
    /// Create a new sandbox with default settings (python3, 30s timeout).
    pub fn new() -> Self {
        Self {
            python_path: PathBuf::from("python3"),
            timeout: Duration::from_secs(30),
            work_dir: None,
        }
    }

    /// Create a sandbox with explicit python path and timeout.
    pub fn with_config(python_path: PathBuf, timeout: Duration) -> Self {
        Self {
            python_path,
            timeout,
            work_dir: None,
        }
    }

    /// Set the working directory for the subprocess.
    pub fn with_work_dir(mut self, dir: PathBuf) -> Self {
        self.work_dir = Some(dir);
        self
    }

    /// Execute a Python script with the given input piped to stdin.
    ///
    /// Spawns `python3 -c <script>`, writes `input` to stdin, then waits
    /// for the process to finish (or kills it on timeout).
    pub async fn execute_script(
        &self,
        script: &str,
        input: &str,
    ) -> crate::core::Result<PythonResult> {
        use std::process::Stdio;

        let mut cmd = Command::new(&self.python_path);
        cmd.arg("-c")
            .arg(script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        if let Some(ref dir) = self.work_dir {
            cmd.current_dir(dir);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| AgnosaiError::Sandbox(format!("failed to spawn python process: {e}")))?;

        // Write input to stdin, then drop to close the pipe.
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(input.as_bytes()).await.map_err(|e| {
                AgnosaiError::Sandbox(format!("failed to write to python stdin: {e}"))
            })?;
            // stdin is dropped here, closing the pipe
        }

        // Wait for completion with timeout. `wait_with_output` consumes child,
        // but `kill_on_drop` ensures cleanup if the timeout fires and we drop it.
        match tokio::time::timeout(self.timeout, child.wait_with_output()).await {
            Ok(Ok(output)) => {
                let exit_code = output.status.code().unwrap_or(-1);
                Ok(PythonResult {
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                    exit_code,
                    timed_out: false,
                })
            }
            Ok(Err(e)) => Err(AgnosaiError::Sandbox(format!(
                "python process I/O error: {e}"
            ))),
            Err(_) => {
                // Timeout — child is dropped here, kill_on_drop sends SIGKILL.
                Ok(PythonResult {
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: -1,
                    timed_out: true,
                })
            }
        }
    }

    /// Execute a Python tool using the standard bridge protocol.
    ///
    /// The tool source must define a `create_tool()` function that returns an
    /// object with an `execute(params)` method. The bridge:
    ///
    /// 1. Reads JSON from stdin: `{"tool_name": "...", "parameters": {...}}`
    /// 2. Exec's the tool source code
    /// 3. Calls `create_tool().execute(parameters)`
    /// 4. Writes JSON to stdout: `{"result": ..., "success": true/false, "error": null}`
    pub async fn execute_tool(
        &self,
        tool_source: &str,
        tool_name: &str,
        parameters: &serde_json::Value,
    ) -> crate::core::Result<PythonResult> {
        let wrapper_script = format!(
            r#"
import sys, json, traceback

input_data = json.loads(sys.stdin.read())

try:
    # Execute the tool source to define create_tool()
    exec("""{tool_source}""")

    tool = create_tool()
    result = tool.execute(input_data["parameters"])
    print(json.dumps({{"result": result, "success": True, "error": None}}))
except Exception as e:
    print(json.dumps({{"result": None, "success": False, "error": str(e)}}))
    sys.exit(1)
"#,
            tool_source = tool_source.replace('\\', "\\\\").replace('"', r#"\""#),
        );

        let input = serde_json::json!({
            "tool_name": tool_name,
            "parameters": parameters,
        });
        let input_str = serde_json::to_string(&input)?;

        self.execute_script(&wrapper_script, &input_str).await
    }
}

impl Default for PythonSandbox {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Check if python3 is available; skip test if not.
    async fn python3_available() -> bool {
        Command::new("python3")
            .arg("--version")
            .output()
            .await
            .is_ok()
    }

    #[tokio::test]
    async fn test_echo_script() {
        if !python3_available().await {
            eprintln!("python3 not found, skipping test");
            return;
        }

        let sandbox = PythonSandbox::new();
        let result = sandbox
            .execute_script("print(input())", "hello world")
            .await
            .expect("execute_script should succeed");

        assert_eq!(result.exit_code, 0);
        assert!(!result.timed_out);
        assert_eq!(result.stdout.trim(), "hello world");
    }

    #[tokio::test]
    async fn test_json_roundtrip() {
        if !python3_available().await {
            eprintln!("python3 not found, skipping test");
            return;
        }

        let sandbox = PythonSandbox::new();
        let script = r#"
import sys, json
data = json.loads(sys.stdin.read())
data["processed"] = True
print(json.dumps(data))
"#;
        let input = serde_json::json!({"key": "value", "count": 42});
        let input_str = serde_json::to_string(&input).unwrap();

        let result = sandbox
            .execute_script(script, &input_str)
            .await
            .expect("execute_script should succeed");

        assert_eq!(result.exit_code, 0);
        assert!(!result.timed_out);

        let output: serde_json::Value =
            serde_json::from_str(result.stdout.trim()).expect("should be valid JSON");
        assert_eq!(output["key"], "value");
        assert_eq!(output["count"], 42);
        assert_eq!(output["processed"], true);
    }

    #[tokio::test]
    async fn test_timeout() {
        if !python3_available().await {
            eprintln!("python3 not found, skipping test");
            return;
        }

        let sandbox = PythonSandbox::with_config(PathBuf::from("python3"), Duration::from_secs(1));

        let result = sandbox
            .execute_script("import time; time.sleep(10)", "")
            .await
            .expect("execute_script should return Ok even on timeout");

        assert!(result.timed_out);
        assert_eq!(result.exit_code, -1);
    }

    #[tokio::test]
    async fn test_nonzero_exit_code() {
        if !python3_available().await {
            eprintln!("python3 not found, skipping test");
            return;
        }

        let sandbox = PythonSandbox::new();
        let result = sandbox
            .execute_script(
                "import sys; print('error output', file=sys.stderr); sys.exit(42)",
                "",
            )
            .await
            .expect("execute_script should succeed even with non-zero exit");

        assert_eq!(result.exit_code, 42);
        assert!(!result.timed_out);
        assert!(result.stderr.contains("error output"));
    }

    #[tokio::test]
    async fn test_execute_tool() {
        if !python3_available().await {
            eprintln!("python3 not found, skipping test");
            return;
        }

        let sandbox = PythonSandbox::new();
        let tool_source = r#"
class MyTool:
    def execute(self, params):
        return {"doubled": params["value"] * 2}

def create_tool():
    return MyTool()
"#;
        let params = serde_json::json!({"value": 21});

        let result = sandbox
            .execute_tool(tool_source, "my_tool", &params)
            .await
            .expect("execute_tool should succeed");

        assert_eq!(result.exit_code, 0);
        assert!(!result.timed_out);

        let output: serde_json::Value =
            serde_json::from_str(result.stdout.trim()).expect("should be valid JSON");
        assert_eq!(output["success"], true);
        assert_eq!(output["result"]["doubled"], 42);
        assert!(output["error"].is_null());
    }
}
