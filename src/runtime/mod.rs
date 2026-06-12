// Sentinel Runtime: Ollama model management

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    pub size: u64,
    pub context_length: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub ram_mb: u64,
    pub gpu_available: bool,
    pub vram_mb: Option<u64>,
}

#[derive(Clone)]
pub struct OllamaRuntime {
    pub ollama_host: String,
}

impl OllamaRuntime {
    pub fn new(ollama_host: &str) -> Self {
        OllamaRuntime {
            ollama_host: ollama_host.to_string(),
        }
    }

    pub async fn detect_ollama() -> bool {
        match reqwest::Client::new()
            .get("http://localhost:11434/api/tags")
            .send()
            .await
        {
            Ok(resp) => resp.status() == 200,
            Err(_) => false,
        }
    }

    pub async fn list_models() -> anyhow::Result<Vec<OllamaModel>> {
        let resp = reqwest::Client::new()
            .get("http://localhost:11434/api/tags")
            .send()
            .await?;
        
        let body: serde_json::Value = resp.json().await?;
        let mut models = Vec::new();

        if let Some(model_list) = body["models"].as_array() {
            for m in model_list {
                if let Some(name) = m["name"].as_str() {
                    // Fetch context length for each model
                    let context_length = Self::get_model_context(name).await.unwrap_or(2048);
                    
                    models.push(OllamaModel {
                        name: name.to_string(),
                        size: m["size"].as_u64().unwrap_or(0),
                        context_length,
                    });
                }
            }
        }

        Ok(models)
    }

    /// Fetch context window size for a specific model
    pub async fn get_model_context(model_name: &str) -> anyhow::Result<u32> {
        let client = reqwest::Client::new();
        let resp = client
            .get("http://localhost:11434/api/show")
            .json(&serde_json::json!({"name": model_name}))
            .send()
            .await?;
        
        let body: serde_json::Value = resp.json().await?;
        
        // Try to get context length from llama parameters
        let context_length = body["model_info"]["llama.context_length"]
            .as_u64()
            .or_else(|| body["model_info"]["llama.context_length"].as_i64().map(|x| x as u64))
            .unwrap_or(2048);

        Ok(context_length as u32)
    }

    pub async fn load_model(_name: &str) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn unload_model(_name: &str) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn pull_model(name: &str) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        let _resp = client
            .post("http://localhost:11434/api/pull")
            .json(&serde_json::json!({"name": name}))
            .send()
            .await?;
        Ok(())
    }

    pub async fn delete_model(name: &str) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        let _resp = client
            .delete("http://localhost:11434/api/delete")
            .json(&serde_json::json!({"name": name}))
            .send()
            .await?;
        Ok(())
    }

    pub async fn switch_model(_name: &str) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn detect_hardware() -> HardwareInfo {
        HardwareInfo {
            ram_mb: 8192,
            gpu_available: false,
            vram_mb: None,
        }
    }

    pub fn recommend_models(hw: &HardwareInfo) -> Vec<(&'static str, u32)> {
        // Returns (model_name, context_length) tuples
        if hw.ram_mb < 2048 {
            vec![("tinyllama:1.1b", 2048), ("qwen:1.8b", 2048)]
        } else if hw.ram_mb < 4096 {
            vec![("gemma:2b", 4096), ("phi3:mini", 4096)]
        } else if hw.ram_mb < 16384 {
            vec![("mistral:7b", 8192), ("qwen:7b", 8192)]
        } else {
            vec![("mixtral:8x7b", 16384), ("qwen:14b", 16384)]
        }
    }
}
