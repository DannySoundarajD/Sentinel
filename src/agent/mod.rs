// Sentinel Agent: Orchestration loop for Ollama

pub mod dispatcher;
pub mod memory_loader;
pub mod prompt;

pub use dispatcher::Dispatcher;

#[derive(Debug, Clone)]
pub struct Agent {
    pub name: String,
    pub model: String,
}

impl Agent {
    pub fn new(name: String, model: String) -> Self {
        Agent { name, model }
    }
}

pub async fn run(agent: Agent, message: String) -> anyhow::Result<String> {
    // Orchestrate: memory_loader → prompt assembly → Ollama dispatch → response
    Ok(format!("Agent {} responded to: {}", agent.name, message))
}
