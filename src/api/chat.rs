use axum::{
    extract::State,
    http::StatusCode,
    response::{sse::Event, IntoResponse, Response, Sse},
    Json,
};
use futures::stream::{self, Stream};
use serde_json::{json, Value};
use std::convert::Infallible;
use std::time::SystemTime;
use uuid::Uuid;
use std::sync::Arc;

use super::{AppState, ChatMessage};
use crate::vault::TokenBudget;

pub async fn health() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "version": "0.1.0",
        "ollama": true,
        "vault": "lite"
    }))
}

#[derive(serde::Deserialize)]
pub struct SendChatRequest {
    pub message: String,
}

pub async fn send(
    State(state): State<AppState>,
    Json(payload): Json<SendChatRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, (StatusCode, String)> {
    let message = payload.message;
    let user_id = Uuid::new_v4().to_string();
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    // FIX 2: Check if model is loaded before processing
    let config = state.config.lock().await;
    let active_model = config.runtime.default_model.clone();
    if active_model.is_empty() {
        drop(config);
        let error_stream = stream::iter(vec![
            Ok(Event::default().data(json!({"error": "No model loaded. Go to Models tab and load a model first.", "done": true}).to_string())),
        ]);
        return Ok(Sse::new(error_stream));
    }
    drop(config);

    // Add user message to history
    let user_msg = ChatMessage {
        id: user_id.clone(),
        role: "user".to_string(),
        content: message.clone(),
        timestamp,
    };
    state.chat_history.lock().await.push(user_msg);

    // FIX 5: Persist to SQLite
    let vault = state.vault.lock().await;
    let _ = vault.save_chat_message(&user_id, "user", &message, timestamp as i64);
    drop(vault);

    // Build context with budget awareness
    let vault = state.vault.lock().await;
    let budget = TokenBudget::new(4096);
    let context = vault.build_context(&message, Some(&budget))
        .unwrap_or_else(|_| message.clone());
    drop(vault);

    // Stub streaming response with context-aware assembly
    let response_id = Uuid::new_v4().to_string();
    let response_timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let stream = stream::iter(vec![
        Ok(Event::default().data(json!({"token": "Hello from Sentinel! ", "done": false}).to_string())),
        Ok(Event::default().data(json!({"token": "Context-aware response ", "done": false}).to_string())),
        Ok(Event::default().data(json!({"token": "on low-spec hardware.", "done": true}).to_string())),
    ]);

    // FIX 5: Persist assistant response to SQLite
    let response_msg = ChatMessage {
        id: response_id.clone(),
        role: "assistant".to_string(),
        content: "Hello from Sentinel! Context-aware response on low-spec hardware.".to_string(),
        timestamp: response_timestamp,
    };
    state.chat_history.lock().await.push(response_msg);
    let vault = state.vault.lock().await;
    let _ = vault.save_chat_message(
        &response_id,
        "assistant",
        "Hello from Sentinel! Context-aware response on low-spec hardware.",
        response_timestamp as i64,
    );
    drop(vault);

    Ok(Sse::new(stream))
}

pub async fn get_history(
    State(state): State<AppState>,
) -> Json<Vec<ChatMessage>> {
    let history = state.chat_history.lock().await;
    Json(history.clone())
}

pub async fn delete_history(
    State(state): State<AppState>,
) -> Json<Value> {
    state.chat_history.lock().await.clear();
    // FIX 5: Also clear from SQLite
    let vault = state.vault.lock().await;
    let _ = vault.clear_chat_history();
    Json(json!({"success": true}))
}
