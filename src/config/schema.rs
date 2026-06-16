// Sentinel Configuration Schema
// Local-first AI agent platform for Linux desktops

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub agent: AgentConfig,
    pub vault: VaultConfig,
    pub runtime: RuntimeConfig,
    pub guardian: GuardianConfig,
    pub telegram: Option<TelegramConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub model: String,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    pub memory_mode: String, // "pro" or "lite"
    pub db_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    #[serde(default = "default_ollama_host")]
    pub ollama_host: String, // default "http://localhost:11434"
    #[serde(default = "default_model")]
    pub default_model: String,
    #[serde(default = "default_fallback_model")]
    pub fallback_model: String, // dynamic fallback model
    #[serde(default = "default_resource_profile")]
    pub resource_profile: String, // "lite", "balanced", "performance"
}

fn default_ollama_host() -> String { "http://localhost:11434".to_string() }
fn default_model() -> String { "gemma:2b".to_string() }
fn default_fallback_model() -> String { "qwen:0.5b".to_string() }
fn default_resource_profile() -> String { "balanced".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardianConfig {
    pub enable: bool,
    pub interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub allowed_users: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        let config_dir = format!("{}/.local/share/sentinx/sentinel", home);

        Config {
            agent: AgentConfig {
                name: "Sentinel".to_string(),
                model: "gemma:2b".to_string(),
                temperature: 0.7,
            },
            vault: VaultConfig {
                memory_mode: "lite".to_string(),
                db_path: format!("{}/vault.db", config_dir).into(),
            },
            runtime: RuntimeConfig {
                ollama_host: "http://localhost:11434".to_string(),
                default_model: "gemma:2b".to_string(),
                fallback_model: "qwen:0.5b".to_string(),
                resource_profile: "balanced".to_string(),
            },
            guardian: GuardianConfig {
                enable: true,
                interval_secs: 10,
            },
            telegram: None,
        }
    }
}

impl Config {
    pub fn load_or_create() -> anyhow::Result<Self> {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        let config_path = format!("{}/.local/share/sentinx/sentinel/config.toml", home);

        let config = if std::path::Path::new(&config_path).exists() {
            let content = std::fs::read_to_string(&config_path)?;
            toml::from_str(&content)?
        } else {
            Config::default()
        };

        // Automatically configure memory mode based on system spec (RAM)
        /*
        let hw = crate::runtime::OllamaRuntime::detect_hardware();
        let mode = if hw.ram_mb >= 15000 { "pro" } else { "lite" };
        if config.vault.memory_mode != mode {
            config.vault.memory_mode = mode.to_string();
            let _ = config.save();
        }
        */

        Ok(config)
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        let config_dir = format!("{}/.local/share/sentinx/sentinel", home);
        std::fs::create_dir_all(&config_dir)?;
        let config_path = format!("{}/config.toml", config_dir);
        let content = toml::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    pub fn vault_db_path(&self) -> String {
        self.vault.db_path.to_string_lossy().to_string()
    }
}

use crate::vault::VaultMode;

impl From<&str> for VaultMode {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pro" => VaultMode::Pro,
            _ => VaultMode::Lite,
        }
    }
}

pub mod traits {
    pub trait ChannelConfig {
        fn name() -> &'static str;
    }
}
