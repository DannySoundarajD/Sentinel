use axum::{extract::State, Json};
use serde_json::{json, Value};

use super::AppState;

pub async fn get_settings(State(state): State<AppState>) -> Json<Value> {
    let config = state.config.lock().await;
    
    Json(json!({
        "ollama_host": config.runtime.ollama_host,
        "default_model": config.runtime.default_model,
        "memory_mode": config.vault.memory_mode,
        "resource_profile": config.runtime.resource_profile,
        "telegram_token": "",
        "telegram_enabled": false
    }))
}

#[derive(serde::Deserialize)]
pub struct UpdateSettingsRequest {
    pub ollama_host: Option<String>,
    pub default_model: Option<String>,
    pub memory_mode: Option<String>,
    pub resource_profile: Option<String>,
    pub telegram_token: Option<String>,
    pub telegram_enabled: Option<bool>,
}

pub async fn update_settings(
    State(state): State<AppState>,
    Json(payload): Json<UpdateSettingsRequest>,
) -> Json<Value> {
    let mut config = state.config.lock().await;
    
    if let Some(host) = payload.ollama_host {
        config.runtime.ollama_host = host;
    }
    if let Some(model) = payload.default_model {
        config.runtime.default_model = model;
    }
    if let Some(mode) = payload.memory_mode {
        config.vault.memory_mode = mode;
    }
    if let Some(profile) = payload.resource_profile {
        config.runtime.resource_profile = profile;
    }
    
    Json(json!({"success": true}))
}
