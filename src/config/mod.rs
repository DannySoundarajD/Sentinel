pub mod schema;
pub mod traits;

pub use schema::Config;

pub fn load_config() -> anyhow::Result<Config> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let config_path = format!("{}/.local/share/sentinx/sentinel/config.toml", home);

    if std::path::Path::new(&config_path).exists() {
        let content = std::fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    } else {
        Ok(Config::default())
    }
}
