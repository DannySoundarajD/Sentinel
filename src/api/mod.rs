use axum::{
    extract::{ws::WebSocketUpgrade, Path, Query, State},
    http::StatusCode,
    response::{sse::Event, IntoResponse, Response, Sse},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

mod chat;
mod guardian;
mod runtime;
mod settings;
mod skills;
mod vault;

use crate::vault::Vault;
use crate::runtime::OllamaRuntime;
use crate::guardian::Guardian;
use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub vault: Arc<Mutex<Vault>>,
    pub runtime: Arc<Mutex<OllamaRuntime>>,
    pub guardian: Arc<Guardian>,
    pub config: Arc<Mutex<Config>>,
    pub chat_history: Arc<Mutex<Vec<ChatMessage>>>,
}

pub use crate::vault::ChatMessage;

pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::very_permissive();

    Router::new()
        // Health check
        .route("/health", get(chat::health))

        // Chat routes
        .route("/chat/send", post(chat::send))
        .route("/chat/history", get(chat::get_history))
        .route("/chat/history", delete(chat::delete_history))
        .route("/chat/session/new", post(chat::start_new_session))

        // Runtime routes
        .route("/runtime/status", get(runtime::get_status))
        .route("/runtime/hardware", get(runtime::get_hardware))
        .route("/runtime/models", get(runtime::list_models))
        .route("/runtime/models/search", get(runtime::search_models))
        .route("/runtime/load", post(runtime::load_model))
        .route("/runtime/unload", post(runtime::unload_model))
        .route("/runtime/pull", post(runtime::pull_model))
        .route("/runtime/switch", post(runtime::switch_model))
        .route("/runtime/model/{name}", delete(runtime::delete_model))
        .route("/runtime/metrics", get(runtime::get_metrics))
        .route("/runtime/proxy/ollama", get(runtime::proxy_ollama))

        // Vault routes
        .route("/vault/nodes", get(vault::get_nodes))
        .route("/vault/edges", get(vault::get_edges))
        .route("/vault/summaries", get(vault::get_summaries))
        .route("/vault/search", get(vault::search_memory))
        .route("/vault/node/{id}", delete(vault::delete_node))
        .route("/vault/edge/{id}", delete(vault::delete_edge))
        .route("/vault/export", get(vault::export_all))
        .route("/vault/save", post(vault::save_memory))
        .route("/vault/summary/{id}", delete(vault::delete_summary))
        .route("/vault/summary/{id}/load", post(vault::load_summary))

        // Guardian routes
        .route("/guardian/status", get(guardian::get_status))
        .route("/guardian/processes", get(guardian::get_processes))
        .route("/guardian/stream", get(guardian::stream_metrics))

        // Skills routes
        .route("/skills", get(skills::get_skills))
        .route("/skills/{name}/enable", post(skills::enable_skill))
        .route("/skills/{name}/disable", post(skills::disable_skill))
        .route("/skills/{name}", delete(skills::delete_skill))

        // Settings routes
        .route("/settings", get(settings::get_settings))
        .route("/settings", post(settings::update_settings))

        .layer(cors)
        .with_state(state)
}