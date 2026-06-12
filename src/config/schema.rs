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
    pub ollama_host: String, // default "http://localhost:11434"
    pub default_model: String,
    pub resource_profile: String, // "lite", "balanced", "performance"
}

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

        if std::path::Path::new(&config_path).exists() {
            let content = std::fs::read_to_string(&config_path)?;
            Ok(toml::from_str(&content)?)
        } else {
            Ok(Config::default())
        }
    }

    pub fn vault_db_path(&self) -> String {
        self.vault.db_path.to_string_lossy().to_string()
    }
}

pub mod traits {
    pub trait ChannelConfig {
        fn name() -> &'static str;
    }
}
