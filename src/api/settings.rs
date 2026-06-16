use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};

use super::AppState;

pub async fn get_settings(State(state): State<AppState>) -> Json<Value> {
    let config = state.config.lock().await;
    let hw = crate::runtime::OllamaRuntime::detect_hardware().await;
    let memory_mode = if config.vault.memory_mode.is_empty() {
        if hw.ram_total_mb >= 15000 { "pro".to_string() } else { "lite".to_string() }
    } else {
        config.vault.memory_mode.clone()
    };
    let context_window = if hw.ram_available_mb < 6144 {
        4096
    } else if hw.ram_available_mb < 12288 {
        4096
    } else if hw.ram_available_mb < 24576 {
        8192
    } else {
        16384
    };
    
    Json(json!({
        "ollama_host": config.runtime.ollama_host,
        "default_model": config.runtime.default_model,
        "fallback_model": config.runtime.fallback_model,
        "memory_mode": memory_mode,
        "context_window": context_window,
        "resource_profile": config.runtime.resource_profile,
        "telegram_token": config.telegram.as_ref().map(|t| t.bot_token.as_str()).unwrap_or(""),
        "telegram_chat_id": config.telegram.as_ref().and_then(|t| t.allowed_users.first().map(|s| s.as_str())).unwrap_or(""),
        "telegram_enabled": config.telegram.is_some(),
        "suggested_memory_mode": hw.recommended_memory_mode,
        "suggested_memory_mode_reason": hw.memory_mode_reason
    }))
}

#[derive(serde::Deserialize)]
pub struct UpdateSettingsRequest {
    pub ollama_host: Option<String>,
    pub default_model: Option<String>,
    pub fallback_model: Option<String>,
    pub memory_mode: Option<String>,
    pub resource_profile: Option<String>,
    pub telegram_token: Option<String>,
    pub telegram_chat_id: Option<String>,
    pub telegram_enabled: Option<bool>,
}

pub async fn update_settings(
    State(state): State<AppState>,
    Json(payload): Json<UpdateSettingsRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let mut config = state.config.lock().await;
    
    if let Some(host) = payload.ollama_host {
        config.runtime.ollama_host = host.clone();
        let mut runtime = state.runtime.lock().await;
        runtime.ollama_host = host;
    }
    if let Some(model) = payload.default_model {
        config.runtime.default_model = model;
    }
    if let Some(model) = payload.fallback_model {
        config.runtime.fallback_model = model;
    }
    
    if let Some(mode) = payload.memory_mode {
        config.vault.memory_mode = mode.clone();
        let mut vault = state.vault.lock().await;
        vault.mode = match mode.as_str() {
            "pro" => crate::vault::VaultMode::Pro,
            _ => crate::vault::VaultMode::Lite,
        };
    }
    
    if let Some(profile) = payload.resource_profile {
        config.runtime.resource_profile = profile;
    }
    
    let telegram_enabled = payload.telegram_enabled.unwrap_or(config.telegram.is_some());
    if telegram_enabled {
        let token = payload.telegram_token
            .clone()
            .or_else(|| config.telegram.as_ref().map(|t| t.bot_token.clone()))
            .unwrap_or_default();
        let chat_id = payload.telegram_chat_id
            .clone()
            .or_else(|| config.telegram.as_ref().and_then(|t| t.allowed_users.first().cloned()))
            .unwrap_or_default();
        
        if !token.is_empty() {
            config.telegram = Some(crate::config::schema::TelegramConfig {
                bot_token: token,
                allowed_users: if chat_id.is_empty() { vec![] } else { vec![chat_id] },
            });
        } else {
            config.telegram = None;
        }
    } else {
        config.telegram = None;
    }
    
    config.save()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save config: {}", e)))?;
    
    Ok(Json(json!({"success": true})))
}
