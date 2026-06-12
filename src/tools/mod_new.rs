// Sentinel Tools: 5 core executable tools
// file_read, file_write, memory_store, memory_retrieve, bash_sh

pub mod file_read;
pub mod file_write;
pub mod memory_store;

pub use file_read::FileReadTool;
pub use file_write::FileWriteTool;
pub use memory_store::MemoryStoreTool;

#[derive(Debug, Clone)]
pub enum ToolKind {
    FileRead,
    FileWrite,
    MemoryStore,
    BashSh,
    MemoryRetrieve,
}

pub struct Tool {
    kind: ToolKind,
}

impl Tool {
    pub async fn execute(&self, args: &str) -> anyhow::Result<String> {
        match self.kind {
            ToolKind::FileRead => {
                let tool = FileReadTool;
                tool.execute(args).await
            }
            ToolKind::FileWrite => {
                let tool = FileWriteTool;
                tool.execute(args).await
            }
            ToolKind::MemoryStore => {
                let tool = MemoryStoreTool;
                tool.execute(args).await
            }
            _ => Err(anyhow::anyhow!("Tool not implemented yet")),
        }
    }
}
