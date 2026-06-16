// Simplified FileWriteTool for Sentinel

use super::traits::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::json;

pub struct FileWriteTool;

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "Write contents to a file"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let path = args["path"]
            .as_str()
            .or_else(|| args["arguments"]["path"].as_str())
            .ok_or_else(|| anyhow::anyhow!("path parameter required"))?;
        let content = args["content"]
            .as_str()
            .or_else(|| args["arguments"]["content"].as_str())
            .ok_or_else(|| anyhow::anyhow!("content parameter required"))?;

        match std::fs::write(path, content) {
            Ok(_) => Ok(ToolResult {
                output: format!("Successfully wrote to {}", path),
                success: true,
            }),
            Err(e) => Ok(ToolResult {
                output: format!("Error writing file: {}", e),
                success: false,
            }),
        }
    }
}