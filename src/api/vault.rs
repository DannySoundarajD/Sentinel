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
    pub key: String,
    pub value: String,
}

pub async fn get_nodes(State(state): State<AppState>) -> Json<Value> {
    Json(json!([]))
}

pub async fn get_edges(State(state): State<AppState>) -> Json<Value> {
    Json(json!([]))
}

pub async fn get_summaries(State(state): State<AppState>) -> Json<Value> {
    Json(json!([]))
}

pub async fn search_memory(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Json<Value> {
    Json(json!([]))
}

pub async fn delete_node(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Json<Value> {
    Json(json!({"success": true}))
}

pub async fn export_all(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "nodes": [],
        "edges": [],
        "summaries": [],
        "exported_at": "2024-01-01T00:00:00Z"
    }))
}

pub async fn save_memory(
    State(state): State<AppState>,
    Json(payload): Json<SaveMemoryRequest>,
) -> Json<Value> {
    Json(json!({"success": true}))
}
