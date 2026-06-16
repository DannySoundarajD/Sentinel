use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use directories::ProjectDirs;

use super::AppState;

pub async fn get_skills(State(state): State<AppState>) -> Json<Value> {
    let skills_dir = ProjectDirs::from("", "", "sentinx")
        .map(|dirs| dirs.data_local_dir().join("skills"))
        .unwrap_or_else(|| PathBuf::from("."));

    let mut skills = Vec::new();

    if let Ok(entries) = fs::read_dir(&skills_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Ok(skill_json) = fs::read_to_string(path.join("skill.json")) {
                    if let Ok(skill_data) = serde_json::from_str::<Value>(&skill_json) {
                        let name = skill_data["name"].as_str().unwrap_or("unknown");
                        let version = skill_data["version"].as_str().unwrap_or("1.0");
                        let enabled = path.join(".enabled").exists();
                        let permissions: Vec<String> = skill_data["permissions"]
                            .as_array()
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|p| p.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default();

                        skills.push(json!({
                            "name": name,
                            "version": version,
                            "enabled": enabled,
                            "permissions": permissions
                        }));
                    }
                }
            }
        }
    }

    Json(Value::Array(skills))
}

pub async fn enable_skill(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<Value> {
    let skills_dir = ProjectDirs::from("", "", "sentinx")
        .map(|dirs| dirs.data_local_dir().join("skills"))
        .unwrap_or_else(|| PathBuf::from("."));
    let marker = skills_dir.join(&name).join(".enabled");
    
    let _ = fs::create_dir_all(skills_dir.join(&name));
    let _ = fs::write(marker, "");
    
    Json(json!({"success": true}))
}

pub async fn disable_skill(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<Value> {
    let skills_dir = ProjectDirs::from("", "", "sentinx")
        .map(|dirs| dirs.data_local_dir().join("skills"))
        .unwrap_or_else(|| PathBuf::from("."));
    let marker = skills_dir.join(&name).join(".enabled");
    
    let _ = fs::remove_file(marker);
    
    Json(json!({"success": true}))
}

pub async fn delete_skill(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<Value> {
    let skills_dir = ProjectDirs::from("", "", "sentinx")
        .map(|dirs| dirs.data_local_dir().join("skills"))
        .unwrap_or_else(|| PathBuf::from("."));
    let skill_path = skills_dir.join(&name);
    
    let _ = fs::remove_dir_all(skill_path);
    
    Json(json!({"success": true}))
}
