// Sentinel Tools: 5 core executable tools

pub mod file_read;
pub mod file_write;
pub mod memory_store;
pub mod bash_exec;
pub mod web_search;
pub mod code_preview;

pub mod traits;

pub use file_read::FileReadTool;
pub use file_write::FileWriteTool;
pub use memory_store::MemoryStoreTool;
pub use bash_exec::BashExecTool;
pub use web_search::WebSearchTool;
pub use code_preview::CodePreviewTool;
pub use traits::Tool;

pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        ToolRegistry { tools: vec![] }
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.push(tool);
    }

    pub fn get_tools(&self) -> &[Box<dyn Tool>] {
        &self.tools
    }
}