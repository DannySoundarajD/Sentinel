use axum::{
    extract::{ws::{WebSocket, WebSocketUpgrade}, Path, State},
    http::StatusCode,
    Json,
};
use futures::SinkExt;
use serde_json::{json, Value};
use tokio::time::{interval, Duration};

use super::AppState;

pub async fn get_status(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "cpu_pct": 23.5,
        "ram_used_mb": 8192,
        "ram_total_mb": 16384,
        "ram_pct": 50.0,
        "gpu_pct": null,
        "vram_used_mb": null,
        "cpu_temp_c": 62.0,
        "top_processes": [
            {
                "name": "firefox",
                "pid": 1234,
                "ram_mb": 512
            },
            {
                "name": "ollama",
                "pid": 5678,
                "ram_mb": 2048
            }
        ],
        "alert": null
    }))
}

pub async fn get_processes(State(state): State<AppState>) -> Json<Value> {
    Json(json!([
        {
            "name": "firefox",
            "pid": 1234,
            "ram_mb": 512
        },
        {
            "name": "ollama",
            "pid": 5678,
            "ram_mb": 2048
        }
    ]))
}

pub async fn stream_metrics(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut interval = interval(Duration::from_secs(10));
    
    loop {
        interval.tick().await;
        
        let metrics = json!({
            "cpu_pct": 23.5,
            "ram_used_mb": 8192,
            "ram_total_mb": 16384,
            "ram_pct": 50.0,
            "gpu_pct": null,
            "vram_used_mb": null,
            "cpu_temp_c": 62.0,
            "top_processes": [],
            "alert": null
        });
        
        if socket.send(axum::extract::ws::Message::Text(metrics.to_string().into())).await.is_err() {
            break;
        }
    }
}
