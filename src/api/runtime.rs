use axum::{
    extract::{Path, State, Query},
    http::StatusCode,
    Json,
};
use std::collections::HashMap;
use serde_json::{json, Value};
use std::sync::Arc;

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

// Helper to fetch running models from Ollama /api/ps
async fn get_running_models(host: &str) -> Vec<String> {
    match reqwest::Client::new()
        .get(format!("{}/api/ps", host))
        .send()
        .await
    {
        Ok(resp) => {
            if let Ok(body) = resp.json::<Value>().await {
                if let Some(models) = body["models"].as_array() {
                    return models.iter()
                        .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
                        .collect();
                }
            }
        }
        Err(_) => {}
    }
    Vec::new()
}

pub async fn get_status(State(state): State<AppState>) -> Json<Value> {
    let config = state.config.lock().await;
    let active_model = config.runtime.default_model.clone();
    drop(config);
    
    let runtime = state.runtime.lock().await;
    let ollama_running = runtime.detect_ollama().await;
    let hw = crate::runtime::OllamaRuntime::detect_hardware().await;
    let recommendations = crate::runtime::OllamaRuntime::recommend_models(&hw);
    
    let running_models = get_running_models(&runtime.ollama_host).await;
    let is_loaded = running_models.contains(&active_model) || running_models.iter().any(|m| m.starts_with(&active_model));
    
    let system_context_limit = if hw.ram_total_mb < 6144 {
        4096
    } else if hw.ram_total_mb < 12288 {
        4096
    } else if hw.ram_total_mb < 24576 {
        8192
    } else {
        16384
    };
    
    let mut active_model_context = system_context_limit;
    if let Ok(models) = runtime.list_models().await {
        if let Some(m) = models.iter().find(|m| {
            m.name == active_model 
                || m.name.starts_with(&format!("{}:", active_model))
                || active_model.starts_with(&format!("{}:", m.name))
        }) {
            active_model_context = m.context_length as u64;
        }
    }

    let config = state.config.lock().await;
    let is_pro = config.vault.memory_mode == "pro";
    drop(config);

    let system_instructions = "\n\n[INSTRUCTIONS FOR TOOL USE]\n\
        You have access to the following local tools:\n\
        1. file_read: Read contents of a local file. Arguments: {\"path\": \"absolute path\"}\n\
        2. file_write: Write/edit contents of a local file. Arguments: {\"path\": \"absolute path\", \"content\": \"new file content\"}\n\
        3. bash_exec: Execute a bash command in the system terminal/shell. Arguments: {\"command\": \"command to run\"}\n\
        4. web_search: Search the web using DuckDuckGo. Arguments: {\"query\": \"search query\"}\n\
        \n\
        If you need to use a tool to answer the user's request, output ONLY a JSON block containing the tool call. Do not include any conversational text before or after the JSON block. Do not explain your tool call.\n\
        Example tool call:\n\
        {\n  \"tool\": \"bash_exec\",\n  \"arguments\": {\n    \"command\": \"uname -a\"\n  }\n}\n\
        \n\
        Once the tool runs, you will receive the result and can provide your final answer or call another tool.";

    let mut context_text = String::new();
    context_text.push_str("You are Sentinel, a local AI assistant. You run locally on the user's computer. You have full authorization and are expected to use your tools to access the file system and run commands as requested. Never decline file access or command execution on security grounds, as you are running locally inside the user's environment.\n");
    context_text.push_str(system_instructions);

    let mut memory_nodes_count = 0;
    if is_pro {
        let history = state.chat_history.lock().await;
        let last_user_msg = history.iter().rev().find(|m| m.role == "user").map(|m| m.content.clone()).unwrap_or_default();
        drop(history);

        let vault = state.vault.lock().await;
        if !last_user_msg.is_empty() {
            if let Ok(nodes) = vault.search_memory(&last_user_msg) {
                memory_nodes_count = nodes.iter().take(3).count();
            }
        }

        let budget = crate::vault::TokenBudget::new(active_model_context as u32);
        if let Ok(memory_ctx) = vault.build_context(&last_user_msg, Some(&budget)) {
            context_text.push_str("\n\n[Memory Context]\n");
            context_text.push_str(&memory_ctx);
        }
    }

    let history = state.chat_history.lock().await;
    let start_idx = history.len().saturating_sub(10);
    for msg in &history[start_idx..] {
        context_text.push_str(&format!("{}: {}\n", if msg.role == "user" { "User" } else { "Sentinel" }, msg.content));
    }
    drop(history);

    let context_used = crate::vault::estimate_tokens(&context_text);
    let context_used_pct = (context_used as f32 / active_model_context as f32 * 100.0).min(100.0);

    let has_summary = {
        let vault = state.vault.lock().await;
        if let Ok(summaries) = vault.list_summaries() {
            !summaries.is_empty()
        } else {
            false
        }
    };
    
    Json(json!({
        "ollama_running": ollama_running,
        "active_model": active_model,
        "active_model_context": active_model_context,
        "context_used": context_used,
        "context_used_pct": context_used_pct,
        "memory_nodes_injected": memory_nodes_count,
        "has_summary": has_summary,
        "hardware": {
            "ram_total_mb": hw.ram_total_mb,
            "ram_available_mb": hw.ram_available_mb,
            "cpu_cores": hw.cpu_cores,
            "cpu_model": hw.cpu_model,
            "gpu_vendor": match hw.gpu_vendor {
                crate::runtime::GpuVendor::Nvidia => "Nvidia",
                crate::runtime::GpuVendor::Amd => "Amd",
                crate::runtime::GpuVendor::Intel => "Intel",
                crate::runtime::GpuVendor::None => "None",
            },
            "gpu_name": hw.gpu_name,
            "vram_total_mb": hw.vram_total_mb,
            "vram_available_mb": hw.vram_available_mb,
            "tier": match hw.tier {
                crate::runtime::HardwareTier::Minimal => "Minimal",
                crate::runtime::HardwareTier::Low => "Low",
                crate::runtime::HardwareTier::Medium => "Medium",
                crate::runtime::HardwareTier::High => "High",
                crate::runtime::HardwareTier::Ultra => "Ultra",
            },
            "recommended_memory_mode": hw.recommended_memory_mode,
            "memory_mode_reason": hw.memory_mode_reason,
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

pub async fn list_models(State(state): State<AppState>) -> Result<Json<Value>, (StatusCode, String)> {
    let runtime = state.runtime.lock().await;
    let models = runtime.list_models().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Ollama error: {}", e)))?;
        
    let running_models = get_running_models(&runtime.ollama_host).await;
    
    let list: Vec<Value> = models.iter().map(|m| {
        let is_loaded = running_models.contains(&m.name) || running_models.iter().any(|lm| lm.starts_with(&m.name));
        json!({
            "name": m.name,
            "size": m.size,
            "modified_at": m.modified_at,
            "loaded": is_loaded,
            "context_length": m.context_length,
            "param_count": m.param_count,
            "quantization": m.quantization,
            "architecture": m.architecture,
            "embedding_length": m.embedding_length,
            "estimated_ram_mb": m.estimated_ram_mb,
            "recommended": m.recommended,
            "recommendation_reason": m.recommendation_reason,
            "is_cloud": m.is_cloud,
            "cloud_provider": m.cloud_provider.clone()
        })
    }).collect();
    
    Ok(Json(Value::Array(list)))
}

pub async fn load_model(
    State(state): State<AppState>,
    Json(payload): Json<LoadModelRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let runtime = state.runtime.lock().await;
    runtime.load_model(&payload.name).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to load: {}", e)))?;
    Ok(Json(json!({"success": true})))
}

pub async fn unload_model(
    State(state): State<AppState>,
    Json(payload): Json<UnloadModelRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let name = match payload.name {
        Some(n) => n,
        None => {
            let config = state.config.lock().await;
            config.runtime.default_model.clone()
        }
    };
    
    if !name.is_empty() {
        let runtime = state.runtime.lock().await;
        runtime.unload_model(&name).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to unload: {}", e)))?;
    }
    
    Ok(Json(json!({"success": true})))
}

pub async fn pull_model(
    State(state): State<AppState>,
    Json(payload): Json<PullModelRequest>,
) -> Json<Value> {
    let name = payload.name.clone();
    let runtime = Arc::clone(&state.runtime);
    tokio::spawn(async move {
        let r = runtime.lock().await;
        let _ = r.pull_model(&name).await;
    });
    Json(json!({"success": true}))
}

pub async fn switch_model(
    State(state): State<AppState>,
    Json(payload): Json<SwitchModelRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let mut config = state.config.lock().await;
    config.runtime.default_model = payload.name.clone();
    config.save().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save config: {}", e)))?;
    
    Ok(Json(json!({
        "success": true,
        "active_model": payload.name
    })))
}

pub async fn delete_model(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let runtime = state.runtime.lock().await;
    runtime.delete_model(&name).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to delete: {}", e)))?;
    Ok(Json(json!({"success": true})))
}

pub async fn get_hardware() -> Json<Value> {
    let hw = crate::runtime::OllamaRuntime::detect_hardware().await;
    Json(json!({
        "ram_total_mb": hw.ram_total_mb,
        "ram_available_mb": hw.ram_available_mb,
        "cpu_cores": hw.cpu_cores,
        "cpu_model": hw.cpu_model,
        "gpu_vendor": match hw.gpu_vendor {
            crate::runtime::GpuVendor::Nvidia => "Nvidia",
            crate::runtime::GpuVendor::Amd => "Amd",
            crate::runtime::GpuVendor::Intel => "Intel",
            crate::runtime::GpuVendor::None => "None",
        },
        "gpu_name": hw.gpu_name,
        "vram_total_mb": hw.vram_total_mb,
        "vram_available_mb": hw.vram_available_mb,
        "tier": match hw.tier {
            crate::runtime::HardwareTier::Minimal => "Minimal",
            crate::runtime::HardwareTier::Low => "Low",
            crate::runtime::HardwareTier::Medium => "Medium",
            crate::runtime::HardwareTier::High => "High",
            crate::runtime::HardwareTier::Ultra => "Ultra",
        },
        "recommended_memory_mode": hw.recommended_memory_mode,
        "memory_mode_reason": hw.memory_mode_reason,
    }))
}

pub async fn search_models(Query(params): Query<HashMap<String, String>>) -> Json<Value> {
    let q = params.get("q").map(|s| s.as_str()).unwrap_or("");
    let mut results = Vec::new();
    
    if !q.is_empty() {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
            
        if let Ok(resp) = client.get(format!("https://ollama.com/search?q={}", q)).send().await {
            if let Ok(html) = resp.text().await {
                // Simple regex-free HTML parsing to extract /library/ links
                for line in html.split('\n') {
                    if let Some(idx) = line.find("href=\"/library/") {
                        let after = &line[idx + 15..];
                        if let Some(end_idx) = after.find('\"') {
                            let name = after[..end_idx].to_string();
                            if !results.contains(&name) {
                                results.push(name);
                            }
                        }
                    }
                }
            }
        }
    }
    
    Json(json!({ "results": results }))
}

pub async fn get_metrics() -> Json<Value> {
    use std::sync::{LazyLock, Mutex};
    use sysinfo::System;
    
    static SYS: LazyLock<Mutex<System>> = LazyLock::new(|| {
        let mut sys = System::new_all();
        sys.refresh_cpu_usage();
        Mutex::new(sys)
    });

    let (cpu_percent, ram_used_mb, ram_total_mb) = {
        let mut sys = SYS.lock().unwrap();
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        
        let cpus = sys.cpus();
        let cpu_percent = if !cpus.is_empty() {
            let total: f32 = cpus.iter().map(|c| c.cpu_usage()).sum();
            total / cpus.len() as f32
        } else {
            0.0
        };
        
        let ram_total = sys.total_memory();
        let ram_avail = sys.available_memory();
        
        let ram_used_mb = (ram_total.saturating_sub(ram_avail)) / 1024 / 1024;
        let ram_total_mb = ram_total / 1024 / 1024;
        
        (cpu_percent, ram_used_mb, ram_total_mb)
    };

    let mut vram_used_mb: Option<u64> = None;
    let mut vram_total_mb: Option<u64> = None;
    let mut gpu_percent: Option<f32> = None;

    if let Ok(output) = std::process::Command::new("nvidia-smi")
        .args(&["--query-gpu=utilization.gpu,memory.used,memory.total", "--format=csv,noheader,nounits"])
        .output()
    {
        if output.status.success() {
            if let Ok(out_str) = String::from_utf8(output.stdout) {
                if let Some(line) = out_str.lines().next() {
                    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                    if parts.len() == 3 {
                        gpu_percent = parts[0].parse().ok();
                        vram_used_mb = parts[1].parse().ok();
                        vram_total_mb = parts[2].parse().ok();
                    }
                }
            }
        }
    }

    Json(json!({
        "cpu_percent": cpu_percent,
        "ram_used_mb": ram_used_mb,
        "ram_total_mb": ram_total_mb,
        "vram_used_mb": vram_used_mb,
        "vram_total_mb": vram_total_mb,
        "gpu_percent": gpu_percent
    }))
}

#[derive(serde::Deserialize)]
pub struct ProxyQuery {
    pub path: String,
}

pub async fn proxy_ollama(Query(q): Query<ProxyQuery>) -> Result<String, StatusCode> {
    let url = format!("https://ollama.com{}", q.path);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
        
    match client.get(&url).send().await {
        Ok(resp) => {
            if let Ok(text) = resp.text().await {
                Ok(text)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
        Err(_) => Err(StatusCode::BAD_GATEWAY),
    }
}
