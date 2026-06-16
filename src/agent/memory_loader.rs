// Memory Loader for Vault integration

pub struct MemoryLoader;

impl MemoryLoader {
    pub async fn load_context(query: &str) -> anyhow::Result<String> {
        Ok(format!("Context for: {}", query))
    }
}
