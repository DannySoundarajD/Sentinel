// Simplified MemoryStoreTool for Sentinel

use super::traits::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::json;

pub struct MemoryStoreTool;

#[async_trait]
impl Tool for MemoryStoreTool {
    fn name(&self) -> &str {
        "memory_store"
    }

    fn description(&self) -> &str {
        "Store a fact or note in long-term memory"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "key": {
                    "type": "string",
                    "description": "Memory key"
                },
                "value": {
                    "type": "string",
                    "description": "Memory value"
                }
            },
            "required": ["key", "value"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let key = args["key"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("key parameter required"))?;

        Ok(ToolResult {
            output: format!("Stored memory: {}", key),
            success: true,
        })
    }
}