use axum::{
    extract::{ws::{WebSocket, WebSocketUpgrade}, State},
    http::StatusCode,
    Json,
};
use futures::SinkExt;
use serde_json::{json, Value};
use tokio::time::{interval, Duration};

use super::AppState;

pub async fn get_status(State(state): State<AppState>) -> Result<Json<Value>, (StatusCode, String)> {
    let metrics = state.guardian.collect_metrics().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        
    let alert = match crate::guardian::Guardian::check_alerts(&metrics) {
        Some(crate::guardian::RamAlert::Critical) => Some("Critical RAM usage! System might swap."),
        Some(crate::guardian::RamAlert::Warning) => Some("High RAM usage warning."),
        None => None,
    };

    Ok(Json(json!({
        "cpu_pct": metrics.cpu_pct,
        "ram_used_mb": metrics.ram_used_mb,
        "ram_total_mb": metrics.ram_total_mb,
        "ram_pct": metrics.ram_pct,
        "gpu_pct": metrics.gpu_pct,
        "vram_used_mb": metrics.vram_used_mb,
        "cpu_temp_c": metrics.cpu_temp_c,
        "top_processes": metrics.top_processes.iter().map(|p| json!({
            "name": p.name,
            "pid": p.pid,
            "ram_mb": p.ram_mb
        })).collect::<Vec<_>>(),
        "alert": alert
    })))
}

pub async fn get_processes(State(state): State<AppState>) -> Result<Json<Value>, (StatusCode, String)> {
    let metrics = state.guardian.collect_metrics().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        
    let list = metrics.top_processes.iter().map(|p| json!({
        "name": p.name,
        "pid": p.pid,
        "ram_mb": p.ram_mb
    })).collect::<Vec<_>>();
    
    Ok(Json(Value::Array(list)))
}

pub async fn stream_metrics(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut interval = interval(Duration::from_secs(2));
    
    loop {
        interval.tick().await;
        
        let metrics_res = state.guardian.collect_metrics().await;
        if let Ok(metrics) = metrics_res {
            let alert = match crate::guardian::Guardian::check_alerts(&metrics) {
                Some(crate::guardian::RamAlert::Critical) => Some("Critical RAM usage! System might swap."),
                Some(crate::guardian::RamAlert::Warning) => Some("High RAM usage warning."),
                None => None,
            };

            let payload = json!({
                "cpu_pct": metrics.cpu_pct,
                "ram_used_mb": metrics.ram_used_mb,
                "ram_total_mb": metrics.ram_total_mb,
                "ram_pct": metrics.ram_pct,
                "gpu_pct": metrics.gpu_pct,
                "vram_used_mb": metrics.vram_used_mb,
                "cpu_temp_c": metrics.cpu_temp_c,
                "top_processes": metrics.top_processes.iter().map(|p| json!({
                    "name": p.name,
                    "pid": p.pid,
                    "ram_mb": p.ram_mb
                })).collect::<Vec<_>>(),
                "alert": alert
            });
            
            if socket.send(axum::extract::ws::Message::Text(payload.to_string().into())).await.is_err() {
                break;
            }
        }
    }
}
