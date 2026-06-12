use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};

use super::AppState;

#[derive(serde::Deserialize)]
pub struct LoadModelRequest {
    pub name: String,
}

#[derive(serde::Deserialize)]
pub struct UnloadModelRequest {
    pub name: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct PullModelRequest {
    pub name: String,
}

#[derive(serde::Deserialize)]
pub struct SwitchModelRequest {
    pub name: String,
}

pub async fn get_status(State(state): State<AppState>) -> Json<Value> {
    let config = state.config.lock().await;
    let active_model = config.runtime.default_model.as_str();
    let hw = crate::runtime::OllamaRuntime::detect_hardware();
    let recommendations = crate::runtime::OllamaRuntime::recommend_models(&hw);
    
    // Find context length of active model (default to 2048)
    let active_model_context = recommendations
        .iter()
        .find(|(name, _)| name == &active_model)
        .map(|(_, ctx)| *ctx)
        .unwrap_or(2048);
    
    Json(json!({
        "ollama_running": true,
        "active_model": active_model,
        "active_model_context": active_model_context,
        "context_used_pct": 34.0,
        "hardware": {
            "ram_total_mb": hw.ram_mb,
            "gpu_available": hw.gpu_available,
            "vram_mb": hw.vram_mb
        },
        "recommended_models": recommendations
            .iter()
            .map(|(name, ctx)| json!({
                "name": name,
                "context_length": ctx
            }))
            .collect::<Vec<_>>()
    }))
}

pub async fn list_models(State(state): State<AppState>) -> Json<Value> {
    Json(json!([
        {
            "name": "gemma:2b",
            "size": 1600000000,
            "modified_at": "2024-01-01T00:00:00Z",
            "loaded": false,
            "context_length": 4096
        }
    ]))
}

pub async fn load_model(
    State(state): State<AppState>,
    Json(payload): Json<LoadModelRequest>,
) -> Json<Value> {
    Json(json!({"success": true}))
}

pub async fn unload_model(
    State(state): State<AppState>,
    Json(payload): Json<UnloadModelRequest>,
) -> Json<Value> {
    Json(json!({"success": true}))
}

pub async fn pull_model(
    State(state): State<AppState>,
    Json(payload): Json<PullModelRequest>,
) -> Json<Value> {
    Json(json!({"success": true}))
}

pub async fn switch_model(
    State(state): State<AppState>,
    Json(payload): Json<SwitchModelRequest>,
) -> Json<Value> {
    let mut config = state.config.lock().await;
    config.runtime.default_model = payload.name.clone();
    
    Json(json!({
        "success": true,
        "active_model": payload.name
    }))
}

pub async fn delete_model(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<Value> {
    Json(json!({"success": true}))
}
