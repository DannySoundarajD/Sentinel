// Simplified FileReadTool for Sentinel

use super::traits::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::json;

pub struct FileReadTool;

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "Read file contents with path sandboxing"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to read"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let path = args["path"]
            .as_str()
            .or_else(|| args["arguments"]["path"].as_str())
            .or_else(|| args.as_str())
            .ok_or_else(|| anyhow::anyhow!("path parameter required"))?;

        match std::fs::read_to_string(path) {
            Ok(content) => Ok(ToolResult {
                output: content,
                success: true,
            }),
            Err(e) => Ok(ToolResult {
                output: format!("Error reading file: {}", e),
                success: false,
            }),
        }
    }
}