use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use super::AppState;

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

#[derive(serde::Deserialize)]
pub struct SaveMemoryRequest {
    pub content: String,
}

pub async fn get_nodes(State(state): State<AppState>) -> Result<Json<Value>, (StatusCode, String)> {
    let vault = state.vault.lock().await;
    let nodes = vault.list_nodes()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)))?;
        
    let mapped = nodes.iter().map(|n| {
        json!({
            "id": n.id,
            "name": n.name,
            "type": n.node_type,
            "description": n.description
        })
    }).collect::<Vec<_>>();
    
    Ok(Json(Value::Array(mapped)))
}

pub async fn get_edges(State(state): State<AppState>) -> Result<Json<Value>, (StatusCode, String)> {
    let vault = state.vault.lock().await;
    let edges = vault.list_edges()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)))?;
        
    let mapped = edges.iter().map(|e| {
        json!({
            "id": e.id,
            "source_id": e.source_id,
            "target_id": e.target_id,
            "relation": e.relation
        })
    }).collect::<Vec<_>>();
    
    Ok(Json(Value::Array(mapped)))
}

pub async fn get_summaries(State(state): State<AppState>) -> Result<Json<Value>, (StatusCode, String)> {
    let vault = state.vault.lock().await;
    let summaries = vault.list_summaries()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)))?;
        
    let mapped = summaries.iter().map(|s| {
        json!({
            "id": s.id,
            "title": s.title,
            "summary": s.summary,
            "timestamp": s.timestamp
        })
    }).collect::<Vec<_>>();
    
    Ok(Json(Value::Array(mapped)))
}

pub async fn search_memory(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let vault = state.vault.lock().await;
    let nodes = vault.search_memory(&params.q)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)))?;
        
    let mapped = nodes.iter().map(|n| {
        json!({
            "id": n.id,
            "name": n.name,
            "type": n.node_type,
            "description": n.description
        })
    }).collect::<Vec<_>>();
    
    Ok(Json(Value::Array(mapped)))
}

pub async fn delete_node(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let vault = state.vault.lock().await;
    vault.delete_node(id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to delete node: {}", e)))?;
    Ok(Json(json!({"success": true})))
}

pub async fn delete_edge(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let vault = state.vault.lock().await;
    vault.delete_edge(id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to delete edge: {}", e)))?;
    Ok(Json(json!({"success": true})))
}

pub async fn export_all(State(state): State<AppState>) -> Result<Json<Value>, (StatusCode, String)> {
    let vault = state.vault.lock().await;
    let nodes = vault.list_nodes().unwrap_or_default();
    let edges = vault.list_edges().unwrap_or_default();
    let summaries = vault.list_summaries().unwrap_or_default();
    
    let now = chrono::Local::now().to_rfc3339();
    
    Ok(Json(json!({
        "nodes": nodes.iter().map(|n| json!({
            "id": n.id,
            "name": n.name,
            "type": n.node_type,
            "description": n.description
        })).collect::<Vec<_>>(),
        "edges": edges.iter().map(|e| json!({
            "id": e.id,
            "source_id": e.source_id,
            "target_id": e.target_id,
            "relation": e.relation
        })).collect::<Vec<_>>(),
        "summaries": summaries.iter().map(|s| json!({
            "id": s.id,
            "title": s.title,
            "summary": s.summary,
            "timestamp": s.timestamp
        })).collect::<Vec<_>>(),
        "exported_at": now
    })))
}

pub async fn save_memory(
    State(state): State<AppState>,
    Json(payload): Json<SaveMemoryRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let vault = state.vault.lock().await;
    vault.save_memory_node(&payload.content, "user", &payload.content)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save memory: {}", e)))?;
    Ok(Json(json!({"success": true})))
}

pub async fn delete_summary(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let vault = state.vault.lock().await;
    vault.delete_summary(id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to delete summary: {}", e)))?;
    Ok(Json(json!({"success": true})))
}

pub async fn load_summary(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let vault = state.vault.lock().await;
    let summary = vault.get_summary(id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to fetch summary: {}", e)))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Summary not found".to_string()))?;

    if let Some(ref messages_json) = summary.messages {
        if let Ok(messages) = serde_json::from_str::<Vec<super::ChatMessage>>(messages_json) {
            // Update in-memory chat history
            let mut chat_history = state.chat_history.lock().await;
            chat_history.clear();
            *chat_history = messages.clone();

            // Update SQLite chat_history table
            vault.clear_chat_history()
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to clear DB history: {}", e)))?;

            for msg in &messages {
                let _ = vault.persist_chat_message(
                    &msg.id,
                    Some(id),
                    &msg.role,
                    &msg.content,
                    msg.timestamp as i64,
                );
            }

            // Set the active summary ID
            let mut active_id = super::chat::active_summary_id().lock().await;
            *active_id = Some(id);

            return Ok(Json(json!({"success": true, "messages": messages})));
        }
    }

    Err((StatusCode::BAD_REQUEST, "No messages stored in this session".to_string()))
}

/// Get all chat history (including both UI and Telegram messages)
pub async fn get_all_chat_history(
    State(state): State<AppState>,
) -> Result<Json<Vec<super::ChatMessage>>, (StatusCode, String)> {
    let vault = state.vault.lock().await;
    let messages = vault.load_chat_history(1000)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to load chat history: {}", e)))?;
    
    Ok(Json(messages))
}
