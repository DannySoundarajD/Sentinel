// Sentinel Ollama Provider: Simplified implementation

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub message: ChatMessage,
}

pub struct OllamaProvider;

impl OllamaProvider {
    pub async fn generate(model: &str, prompt: &str) -> anyhow::Result<String> {
        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "model": model,
            "prompt": prompt,
        });

        let response = client
            .post("http://localhost:11434/api/generate")
            .json(&payload)
            .send()
            .await?;

        let body: serde_json::Value = response.json().await?;
        Ok(body["response"].as_str().unwrap_or("").to_string())
    }

    pub async fn list_models() -> anyhow::Result<Vec<String>> {
        let client = reqwest::Client::new();
        let response = client
            .get("http://localhost:11434/api/tags")
            .send()
            .await?;

        let body: serde_json::Value = response.json().await?;
        let models: Vec<String> = body["models"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
            .collect();

        Ok(models)
    }
}