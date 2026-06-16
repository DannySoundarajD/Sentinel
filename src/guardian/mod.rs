// Sentinel Guardian: Resource monitoring

use serde::Serialize;
use std::fs;

#[derive(Debug, Clone, Serialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub ram_mb: u64, // frontend API is mapping ram_mb, let's make sure it matches
}

#[derive(Debug, Clone, Serialize)]
pub struct GuardianMetrics {
    pub cpu_pct: f32,
    pub ram_used_mb: u64,
    pub ram_total_mb: u64,
    pub ram_pct: f32, // RAM usage percent
    pub gpu_pct: Option<f32>,
    pub vram_used_mb: Option<u64>,
    pub cpu_temp_c: Option<f32>,
    pub top_processes: Vec<ProcessInfo>,
}

pub enum RamAlert {
    Warning,   // 90%
    Critical,  // 95%
}

#[derive(Clone)]
pub struct Guardian;

async fn read_cpu_stats() -> Option<(u64, u64)> {
    if let Ok(content) = fs::read_to_string("/proc/stat") {
        if let Some(line) = content.lines().next() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 5 {
                let user: u64 = parts[1].parse().unwrap_or(0);
                let nice: u64 = parts[2].parse().unwrap_or(0);
                let system: u64 = parts[3].parse().unwrap_or(0);
                let idle: u64 = parts[4].parse().unwrap_or(0);
                let iowait: u64 = if parts.len() >= 6 { parts[5].parse().unwrap_or(0) } else { 0 };
                let irq: u64 = if parts.len() >= 7 { parts[6].parse().unwrap_or(0) } else { 0 };
                let softirq: u64 = if parts.len() >= 8 { parts[7].parse().unwrap_or(0) } else { 0 };
                let steal: u64 = if parts.len() >= 9 { parts[8].parse().unwrap_or(0) } else { 0 };
                
                let idle_time = idle + iowait;
                let total_time = user + nice + system + idle_time + irq + softirq + steal;
                return Some((idle_time, total_time));
            }
        }
    }
    None
}

pub fn get_top_processes() -> Vec<ProcessInfo> {
    let mut processes = Vec::new();
    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if let Ok(pid) = name.parse::<u32>() {
                    // Try to get process name
                    let name = if let Ok(comm) = fs::read_to_string(path.join("comm")) {
                        comm.trim().to_string()
                    } else {
                        continue;
                    };
                    
                    if name.is_empty() {
                        continue;
                    }

                    // Try to get resident memory usage (RSS) from statm
                    if let Ok(statm) = fs::read_to_string(path.join("statm")) {
                        let parts: Vec<&str> = statm.split_whitespace().collect();
                        if parts.len() >= 2 {
                            if let Ok(rss_pages) = parts[1].parse::<u64>() {
                                let ram_mb = (rss_pages * 4) / 1024; // assuming 4KB page size
                                if ram_mb > 1 { // Filter out tiny processes
                                    processes.push(ProcessInfo { pid, name, ram_mb });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    processes.sort_by(|a, b| b.ram_mb.cmp(&a.ram_mb));
    processes.truncate(8);
    processes
}

impl Guardian {
    pub fn new() -> Self {
        Guardian
    }

    pub async fn collect_metrics(&self) -> anyhow::Result<GuardianMetrics> {
        let (idle1, total1) = read_cpu_stats().await.unwrap_or((0, 0));
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        let (idle2, total2) = read_cpu_stats().await.unwrap_or((0, 0));
        
        let idle_delta = idle2.saturating_sub(idle1);
        let total_delta = total2.saturating_sub(total1);
        
        let cpu_pct = if total_delta > 0 {
            100.0 * (1.0 - (idle_delta as f32 / total_delta as f32))
        } else {
            0.0
        };
        
        // RAM usage
        let mut ram_used_mb = 0;
        let mut ram_total_mb = 8192;
        if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
            let mut total = 0;
            let mut available = 0;
            for line in meminfo.lines() {
                if line.starts_with("MemTotal:") {
                    if let Some(val) = line.split_whitespace().nth(1) {
                        total = val.parse::<u64>().unwrap_or(0);
                    }
                } else if line.starts_with("MemAvailable:") {
                    if let Some(val) = line.split_whitespace().nth(1) {
                        available = val.parse::<u64>().unwrap_or(0);
                    }
                }
            }
            if total > 0 {
                ram_total_mb = total / 1024;
                ram_used_mb = (total - available) / 1024;
            }
        }
        
        let ram_pct = if ram_total_mb > 0 {
            (ram_used_mb as f32 / ram_total_mb as f32) * 100.0
        } else {
            0.0
        };
        
        // CPU Temperature
        let mut cpu_temp_c = None;
        for zone in 0..5 {
            let path = format!("/sys/class/thermal/thermal_zone{}/temp", zone);
            if let Ok(temp_str) = fs::read_to_string(path) {
                if let Ok(temp_val) = temp_str.trim().parse::<f32>() {
                    cpu_temp_c = Some(temp_val / 1000.0);
                    break;
                }
            }
        }
        
        // GPU utilization and VRAM usage via nvidia-smi
        let mut gpu_pct = None;
        let mut vram_used_mb = None;
        if let Ok(output) = std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=utilization.gpu,memory.used", "--format=csv,noheader,nounits"])
            .output()
        {
            if output.status.success() {
                let out_str = String::from_utf8_lossy(&output.stdout);
                let parts: Vec<&str> = out_str.trim().split(',').collect();
                if parts.len() >= 2 {
                    if let Ok(gpu) = parts[0].trim().parse::<f32>() {
                        gpu_pct = Some(gpu);
                    }
                    if let Ok(vram) = parts[1].trim().parse::<u64>() {
                        vram_used_mb = Some(vram);
                    }
                }
            }
        }
        
        let top_processes = get_top_processes();
        
        Ok(GuardianMetrics {
            cpu_pct,
            ram_used_mb,
            ram_total_mb,
            ram_pct,
            gpu_pct,
            vram_used_mb,
            cpu_temp_c,
            top_processes,
        })
    }

    pub fn check_alerts(metrics: &GuardianMetrics) -> Option<RamAlert> {
        let ram_pct = (metrics.ram_used_mb as f32 / metrics.ram_total_mb as f32) * 100.0;
        
        if ram_pct >= 95.0 {
            Some(RamAlert::Critical)
        } else if ram_pct >= 90.0 {
            Some(RamAlert::Warning)
        } else {
            None
        }
    }
}
