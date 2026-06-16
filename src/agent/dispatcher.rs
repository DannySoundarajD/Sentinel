// Agent Dispatcher: Routes to Ollama only

pub struct Dispatcher;

impl Dispatcher {
    pub async fn dispatch(model: &str, prompt: &str) -> anyhow::Result<String> {
        // Only Ollama dispatch path
        Ok(format!("Dispatched to {}: {}", model, prompt))
    }
}
