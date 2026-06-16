// Sentinel Runtime: Ollama model management

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    pub size: u64,
    pub modified_at: String,
    pub loaded: bool,
    // Real values fetched from /api/show
    pub context_length: u32,
    pub param_count: Option<u64>,
    pub quantization: Option<String>,
    pub architecture: Option<String>,
    pub embedding_length: Option<u32>,
    // Computed fields
    pub estimated_ram_mb: u64,
    pub recommended: bool,
    pub recommendation_reason: String,
    // Cloud fields
    pub is_cloud: bool,
    pub cloud_provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub ram_total_mb: u64,
    pub ram_available_mb: u64,
    pub cpu_cores: u32,
    pub cpu_model: String,
    pub gpu_vendor: GpuVendor,
    pub gpu_name: Option<String>,
    pub vram_total_mb: Option<u64>,
    pub vram_available_mb: Option<u64>,
    // Computed capability tier
    pub tier: HardwareTier,
    // Recommended memory mode
    pub recommended_memory_mode: String,
    pub memory_mode_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HardwareTier {
    Minimal,    // under 6GB RAM
    Low,        // 6-10GB RAM
    Medium,     // 10-20GB RAM  
    High,       // 20-32GB RAM
    Ultra,      // 32GB+ RAM
}

#[derive(Clone)]
pub struct OllamaRuntime {
    pub ollama_host: String,
}

impl OllamaRuntime {
    pub fn new(ollama_host: &str) -> Self {
        OllamaRuntime {
            ollama_host: ollama_host.to_string(),
        }
    }

    pub async fn detect_ollama(&self) -> bool {
        match reqwest::Client::new()
            .get(format!("{}/api/tags", self.ollama_host))
            .send()
            .await
        {
            Ok(resp) => resp.status() == 200,
            Err(_) => false,
        }
    }

    pub async fn get_loaded_model(&self) -> anyhow::Result<String> {
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{}/api/ps", self.ollama_host))
            .send()
            .await?
            .json::<serde_json::Value>().await?;
        
        let name = resp["models"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|m| m["name"].as_str())
            .unwrap_or("")
            .to_string();
        
        Ok(name)
    }

    pub async fn list_models(&self) -> anyhow::Result<Vec<OllamaModel>> {
        let client = reqwest::Client::new();
        
        // Step 1: Get model list from /api/tags
        let tags_resp = client
            .get(format!("{}/api/tags", self.ollama_host))
            .send().await?
            .json::<serde_json::Value>().await?;
        
        let models_json = tags_resp["models"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        
        // Step 2: Get currently loaded model name
        let loaded_model = self.get_loaded_model().await
            .unwrap_or_default();
        
        let hw_profile = Self::detect_hardware().await;
        
        // Step 3: For each model fetch /api/show
        let mut models = Vec::new();
        for m in models_json {
            let name = m["name"].as_str().unwrap_or("").to_string();
            let size = m["size"].as_u64().unwrap_or(0);
            let modified_at = m["modified_at"]
                .as_str().unwrap_or("").to_string();
            let loaded = name == loaded_model;
            
            // Fetch real model info
            let show_resp = client
                .post(format!("{}/api/show", self.ollama_host))
                .json(&serde_json::json!({"name": name}))
                .send().await;
            
            let (ctx_len, param_count, quantization, architecture, 
                 embedding_length) = match show_resp {
                Ok(resp) => {
                    let data: serde_json::Value = resp.json()
                        .await.unwrap_or_default();
                    let mi = &data["model_info"];
                    let details = &data["details"];
                    
                    // Search all model_info keys for one ending in "context_length"
                    let c_len = mi.as_object()
                        .and_then(|obj| {
                            obj.iter()
                                .find(|(k, _)| k.ends_with("context_length"))
                                .and_then(|(_, v)| v.as_u64())
                        })
                        .unwrap_or(2048) as u32;

                    // Search all model_info keys for one ending in "embedding_length"
                    let emb_len = mi.as_object()
                        .and_then(|obj| {
                            obj.iter()
                                .find(|(k, _)| k.ends_with("embedding_length"))
                                .and_then(|(_, v)| v.as_u64())
                        })
                        .map(|v| v as u32);
                        
                    (
                        c_len,
                        mi["general.parameter_count"].as_u64(),
                        details["quantization_level"]
                            .as_str()
                            .or_else(|| mi["general.quantization_version"].as_str())
                            .map(|s| s.to_string()),
                        mi["general.architecture"]
                            .as_str()
                            .map(|s| s.to_string()),
                        emb_len,
                    )
                }
                Err(_) => (2048, None, None, None, None),
            };
            
            let cloud = Self::is_cloud_model(&name, size);
            let cloud_provider = if cloud {
                Self::detect_cloud_provider(&name)
            } else {
                None
            };
            
            // Estimate RAM: use file size if available, otherwise estimate from param count
            let estimated_ram_mb = if cloud {
                0
            } else if size > 1_000_000 {
                // Use actual file size + 20% overhead
                (size / 1_000_000) * 120 / 100
            } else if let Some(params) = param_count {
                // Estimate from param count
                // BF16/FP16: 2 bytes per param
                // Q4: ~0.5 bytes per param  
                // Q8: ~1 byte per param
                let bytes_per_param: u64 = match quantization
                    .as_deref()
                    .unwrap_or("") {
                    q if q.contains("BF16") || q.contains("FP16") => 2,
                    q if q.contains("Q4") || q == "2" => 1,
                    q if q.contains("Q8") => 1,
                    _ => 2, // safe default
                };
                // params is raw count, convert to MB + 20% overhead
                (params / 1_000_000 * bytes_per_param) * 120 / 100
            } else {
                // Unknown — assume large
                8192
            };
            
            models.push(OllamaModel {
                name,
                size,
                modified_at,
                loaded,
                context_length: ctx_len,
                param_count,
                quantization,
                architecture,
                embedding_length,
                estimated_ram_mb,
                recommended: false,
                recommendation_reason: String::new(),
                is_cloud: cloud,
                cloud_provider,
            });
        }
        
        Self::score_models(&mut models, &hw_profile);
        
        Ok(models)
    }

    pub async fn load_model(&self, name: &str) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        let _resp = client
            .post(format!("{}/api/generate", self.ollama_host))
            .json(&serde_json::json!({
                "model": name,
                "prompt": "",
                "keep_alive": "5m"
            }))
            .send()
            .await?;
        Ok(())
    }

    pub async fn unload_model(&self, name: &str) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        let _resp = client
            .post(format!("{}/api/generate", self.ollama_host))
            .json(&serde_json::json!({
                "model": name,
                "prompt": "",
                "keep_alive": 0
            }))
            .send()
            .await?;
        Ok(())
    }

    pub async fn pull_model(&self, name: &str) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        let _resp = client
            .post(format!("{}/api/pull", self.ollama_host))
            .json(&serde_json::json!({"name": name}))
            .send()
            .await?;
        Ok(())
    }

    pub async fn delete_model(&self, name: &str) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        let _resp = client
            .delete(format!("{}/api/delete", self.ollama_host))
            .json(&serde_json::json!({"name": name}))
            .send()
            .await?;
        Ok(())
    }

    pub async fn switch_model(&self, _name: &str) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn detect_hardware() -> HardwareProfile {
        // RAM from /proc/meminfo
        let meminfo = std::fs::read_to_string("/proc/meminfo")
            .unwrap_or_default();
        
        let ram_total_mb = meminfo.lines()
            .find(|l| l.starts_with("MemTotal:"))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0) / 1024;
        
        let ram_available_mb = meminfo.lines()
            .find(|l| l.starts_with("MemAvailable:"))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0) / 1024;
        
        // CPU
        let cpuinfo = std::fs::read_to_string("/proc/cpuinfo")
            .unwrap_or_default();
        let cpu_model = cpuinfo.lines()
            .find(|l| l.starts_with("model name"))
            .and_then(|l| l.split(':').nth(1))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "Unknown CPU".to_string());
        let cpu_cores = cpuinfo.lines()
            .filter(|l| l.starts_with("processor"))
            .count() as u32;
        
        // GPU — try nvidia-smi first
        let nvidia = tokio::process::Command::new("nvidia-smi")
            .args(["--query-gpu=name,memory.total,memory.free",
                   "--format=csv,noheader,nounits"])
            .output().await;
        
        let mut gpu_vendor = GpuVendor::None;
        let mut gpu_name = None;
        let mut vram_total_mb = None;
        let mut vram_available_mb = None;

        if let Ok(output) = nvidia {
            if output.status.success() {
                let out_str = String::from_utf8_lossy(&output.stdout);
                if let Some(line) = out_str.lines().next() {
                    let parts: Vec<&str> = line.split(',')
                        .map(|s| s.trim()).collect();
                    if parts.len() >= 3 {
                        gpu_vendor = GpuVendor::Nvidia;
                        gpu_name = Some(parts[0].to_string());
                        vram_total_mb = parts[1]
                            .replace("MiB", "")
                            .replace("MB", "")
                            .trim()
                            .parse::<u64>()
                            .ok();
                        vram_available_mb = parts[2]
                            .replace("MiB", "")
                            .replace("MB", "")
                            .trim()
                            .parse::<u64>()
                            .ok();
                    }
                }
            }
        }

        // If Nvidia GPU was not found, check for AMD or Intel via sysfs
        if gpu_vendor == GpuVendor::None {
            if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
                for entry in entries.flatten() {
                    let path = entry.path().join("device");
                    if let (Ok(vendor_str), Ok(device_str)) = (
                        std::fs::read_to_string(path.join("vendor")),
                        std::fs::read_to_string(path.join("device"))
                    ) {
                        let vendor = vendor_str.trim().to_lowercase();
                        let _device = device_str.trim().to_lowercase();
                        if vendor.contains("1002") || vendor.contains("0x1002") {
                            gpu_vendor = GpuVendor::Amd;
                            gpu_name = Some("AMD Radeon GPU".to_string());
                            if let Ok(vram_str) = std::fs::read_to_string(path.join("mem_info_vram_total")) {
                                if let Ok(vram_bytes) = vram_str.trim().parse::<u64>() {
                                    vram_total_mb = Some(vram_bytes / (1024 * 1024));
                                    vram_available_mb = Some(vram_bytes / (1024 * 1024));
                                }
                            }
                            break;
                        } else if vendor.contains("8086") || vendor.contains("0x8086") {
                            gpu_vendor = GpuVendor::Intel;
                            gpu_name = Some("Intel Integrated Graphics".to_string());
                            break;
                        }
                    }
                }
            }
        }

        let tier = if ram_total_mb < 6144 {
            HardwareTier::Minimal
        } else if ram_total_mb < 10240 {
            HardwareTier::Low
        } else if ram_total_mb < 20480 {
            HardwareTier::Medium
        } else if ram_total_mb < 32768 {
            HardwareTier::High
        } else {
            HardwareTier::Ultra
        };

        let recommended_memory_mode = if ram_total_mb >= 15000 {
            "pro".to_string()
        } else {
            "lite".to_string()
        };

        let memory_mode_reason = if ram_total_mb >= 15000 {
            format!("System has {} MB total RAM which is sufficient for Pro Mode knowledge extraction.", ram_total_mb)
        } else {
            format!("System has only {} MB total RAM. Lite Mode is recommended to save resources.", ram_total_mb)
        };

        HardwareProfile {
            ram_total_mb,
            ram_available_mb,
            cpu_cores,
            cpu_model,
            gpu_vendor,
            gpu_name,
            vram_total_mb,
            vram_available_mb,
            tier,
            recommended_memory_mode,
            memory_mode_reason,
        }
    }

    pub fn recommend_models(hw: &HardwareProfile) -> Vec<(String, u32)> {
        // Returns (model_name, context_length) tuples
        if hw.ram_total_mb < 6144 {
            vec![("tinyllama:1.1b".to_string(), 2048), ("qwen:1.8b".to_string(), 2048)]
        } else if hw.ram_total_mb < 10240 {
            vec![("gemma2:2b".to_string(), 4096), ("phi3:mini".to_string(), 4096)]
        } else if hw.ram_total_mb < 20480 {
            vec![("mistral:7b".to_string(), 8192), ("qwen:7b".to_string(), 8192)]
        } else {
            vec![("mixtral:8x7b".to_string(), 16384), ("qwen:14b".to_string(), 16384)]
        }
    }

    pub fn is_cloud_model(name: &str, size: u64) -> bool {
        // Cloud models have no local weights
        // They are API proxies served remotely
        let name_lower = name.to_lowercase();
        name_lower.contains(":cloud")
            || name_lower.contains("-cloud")
            || size < 1_000_000  // less than 1MB = no local weights
    }

    pub fn detect_cloud_provider(name: &str) -> Option<String> {
        let lower = name.to_lowercase();
        if lower.contains("gemma") && lower.contains("cloud") {
            Some("Google (Gemini API)".to_string())
        } else if lower.contains("claude") && lower.contains("cloud") {
            Some("Anthropic".to_string())
        } else if lower.contains("gpt") && lower.contains("cloud") {
            Some("OpenAI".to_string())
        } else if lower.contains("cloud") {
            Some("Cloud API".to_string())
        } else {
            None
        }
    }

    pub fn score_models(
        models: &mut Vec<OllamaModel>,
        hw: &HardwareProfile,
    ) {
        for model in models.iter_mut() {
            // Cloud models are ALWAYS available
            // They need zero local RAM — skip RAM scoring entirely
            if model.is_cloud {
                model.recommended = true;
                model.recommendation_reason = format!(
                    "Cloud model — runs on {} servers, zero local RAM needed. Requires internet connection.",
                    model.cloud_provider
                        .as_deref()
                        .unwrap_or("remote")
                );
                // Reset these — they are meaningless for cloud models
                model.estimated_ram_mb = 0;
                continue;
            }
            
            // Local model scoring — existing logic unchanged
            let ram_budget_mb = hw.ram_available_mb
                .saturating_sub(1024);
            let vram_budget_mb = hw.vram_available_mb
                .unwrap_or(0);
            
            let fits_vram = vram_budget_mb > 0
                && model.estimated_ram_mb <= vram_budget_mb;
            let fits_ram = model.estimated_ram_mb <= ram_budget_mb;
            let fits = fits_vram || fits_ram;
            
            model.recommended = fits;
            model.recommendation_reason = if !fits {
                format!(
                    "Local model needs ~{}MB — only {}MB available",
                    model.estimated_ram_mb,
                    if vram_budget_mb > 0 { vram_budget_mb }
                    else { ram_budget_mb }
                )
            } else if fits_vram {
                format!(
                    "Fits in VRAM (~{}MB needed, {}MB available) — GPU inference",
                    model.estimated_ram_mb,
                    vram_budget_mb
                )
            } else {
                format!(
                    "Fits in RAM (~{}MB needed, {}MB available) — CPU/hybrid inference",
                    model.estimated_ram_mb,
                    ram_budget_mb
                )
            };
        }
        
        // Sort: local recommended first, then cloud, then not recommended
        models.sort_by(|a, b| {
            // Local recommended > cloud > local not recommended
            let a_score = if a.recommended && !a.is_cloud { 2 }
                else if a.is_cloud { 1 }
                else { 0 };
            let b_score = if b.recommended && !b.is_cloud { 2 }
                else if b.is_cloud { 1 }
                else { 0 };
            b_score.cmp(&a_score)
                .then(b.estimated_ram_mb.cmp(&a.estimated_ram_mb))
        });
    }
}
