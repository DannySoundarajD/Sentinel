use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};

use super::AppState;
use crate::tools::CodePreviewTool;

#[derive(serde::Deserialize)]
pub struct CodeExecuteRequest {
    pub language: String,
    pub code: String,
}

#[derive(serde::Deserialize)]
pub struct CodeAnalyzeRequest {
    pub language: String,
    pub code: String,
}

#[derive(serde::Deserialize)]
pub struct CodeFormatRequest {
    pub language: String,
    pub code: String,
}

/// Execute code and return output
pub async fn execute(
    State(_state): State<AppState>,
    Json(payload): Json<CodeExecuteRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let output = CodePreviewTool::execute(&payload.language, &payload.code);
    
    Ok(Json(json!({
        "language": payload.language,
        "success": !output.contains("Error"),
        "output": output
    })))
}

/// Analyze code structure and complexity
pub async fn analyze(
    State(_state): State<AppState>,
    Json(payload): Json<CodeAnalyzeRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let analysis = CodePreviewTool::analyze(&payload.language, &payload.code);
    
    Ok(Json(analysis))
}

/// Format and beautify code
pub async fn format(
    State(_state): State<AppState>,
    Json(payload): Json<CodeFormatRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let formatted = CodePreviewTool::format(&payload.language, &payload.code);
    
    Ok(Json(json!({
        "language": payload.language,
        "original_length": payload.code.len(),
        "formatted_length": formatted.len(),
        "formatted": formatted
    })))
}
