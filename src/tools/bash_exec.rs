use super::traits::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::json;
use std::process::Command;

pub struct BashExecTool;

#[async_trait]
impl Tool for BashExecTool {
    fn name(&self) -> &str {
        "bash_exec"
    }

    fn description(&self) -> &str {
        "Execute a bash command in the local system shell and return output"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The command line string to run"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let command = args["command"]
            .as_str()
            .or_else(|| args["arguments"]["command"].as_str())
            .or_else(|| args.as_str())
            .ok_or_else(|| anyhow::anyhow!("command parameter required"))?;

        match Command::new("bash").arg("-c").arg(command).output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let success = output.status.success();
                
                let combined = if stderr.is_empty() {
                    stdout
                } else if stdout.is_empty() {
                    stderr
                } else {
                    format!("Stdout:\n{}\nStderr:\n{}", stdout, stderr)
                };

                Ok(ToolResult {
                    output: combined,
                    success,
                })
            }
            Err(e) => Ok(ToolResult {
                output: format!("Error running command: {}", e),
                success: false,
            }),
        }
    }
}
