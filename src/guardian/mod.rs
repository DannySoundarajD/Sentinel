// Sentinel Guardian: Resource monitoring

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub rss_mb: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GuardianMetrics {
    pub cpu_pct: f32,
    pub ram_used_mb: u64,
    pub ram_total_mb: u64,
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

impl Guardian {
    pub fn new() -> Self {
        Guardian
    }

    pub fn collect_metrics() -> anyhow::Result<GuardianMetrics> {
        Ok(GuardianMetrics {
            cpu_pct: 0.0,
            ram_used_mb: 512,
            ram_total_mb: 8192,
            gpu_pct: None,
            vram_used_mb: None,
            cpu_temp_c: None,
            top_processes: vec![],
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
