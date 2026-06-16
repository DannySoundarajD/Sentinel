#![warn(clippy::all)]
#![allow(dead_code, unused_variables, unused_imports)]

use clap::{Parser, Subcommand};
use std::sync::Arc;
use tokio::sync::Mutex;

mod agent;
mod api;
mod bridge;
mod config;
mod daemon;
mod guardian;
mod hotkey;
mod notifications;
mod providers;
mod runtime;
mod security;
mod skillforge;
mod skills;
mod tools;
mod tray;
mod vault;

use config::Config;
use vault::Vault;
use runtime::OllamaRuntime;
use guardian::Guardian;

#[derive(Parser)]
#[command(name = "sentinel")]
#[command(about = "Local-first AI agent platform powered by Ollama")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start background service daemon
    Daemon,

    /// Send a message to the active model
    Chat { message: String },

    /// List available Ollama models
    Models,

    /// Search vault memory
    Vault {
        #[command(subcommand)]
        action: VaultAction,
    },

    /// Show resource status and alerts
    Guardian,

    /// List installed skills
    Skills,
}

#[derive(Subcommand)]
enum VaultAction {
    /// Search memory
    Search { query: String },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Daemon) => {
            run_api_server().await?;
        }
        Some(Commands::Chat { message }) => {
            println!("Chat: {}", message);
        }
        Some(Commands::Models) => {
            println!("Available models:");
        }
        Some(Commands::Vault {
            action: VaultAction::Search { query },
        }) => {
            println!("Searching vault for: {}", query);
        }
        Some(Commands::Guardian) => {
            println!("Guardian status:");
        }
        Some(Commands::Skills) => {
            println!("Installed skills:");
        }
        None => {
            println!("Sentinel 0.1.0 - Local-first AI agent for Linux");
            println!("Use 'sentinel --help' for usage information");
        }
    }

    Ok(())
}

async fn run_api_server() -> anyhow::Result<()> {
    // Load configuration
    let config = Config::load_or_create()?;

    // Initialize Vault
    let vault = Vault::new(
        config.vault_db_path().into(),
        match config.vault.memory_mode.as_str() {
            "pro" => crate::vault::VaultMode::Pro,
            _ => crate::vault::VaultMode::Lite,
        },
    )?;

    // Load chat history from SQLite on startup
    let initial_history = vault.load_chat_history(100).unwrap_or_default();

    // Initialize Runtime
    let runtime = OllamaRuntime::new(&config.runtime.ollama_host);

    // Initialize Guardian
    let guardian = Guardian::new();

    // Build AppState
    let state = api::AppState {
        vault: Arc::new(Mutex::new(vault)),
        runtime: Arc::new(Mutex::new(runtime)),
        guardian: Arc::new(guardian),
        config: Arc::new(Mutex::new(config.clone())),
        chat_history: Arc::new(Mutex::new(initial_history)),
    };

    // Initialize Telegram Bridge if enabled
    if let Some(ref tg_config) = config.telegram {
        if !tg_config.bot_token.is_empty() {
            let state_clone = state.clone();
            let token = tg_config.bot_token.clone();
            tokio::spawn(async move {
                let _ = crate::bridge::TelegramBridge::start(state_clone, token).await;
            });
        }
    }

    // Build router and start server
    let app = api::build_router(state);
    
    let mut port = 8888;
    let listener = loop {
        let addr = format!("0.0.0.0:{}", port);
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(l) => break l,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::AddrInUse {
                    println!("Port {} is already in use, trying next port...", port);
                    port += 1;
                    if port > 8999 {
                        return Err(anyhow::anyhow!("Failed to find any free port in range 8888-8999"));
                    }
                } else {
                    return Err(e.into());
                }
            }
        }
    };

    // Write selected port to daemon.port file so frontend can dynamically find it
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let port_file_path = format!("{}/.local/share/sentinx/sentinel/daemon.port", home);
    if let Err(e) = std::fs::write(&port_file_path, port.to_string()) {
        println!("Warning: Failed to write daemon port file: {}", e);
    }
    
    println!("✓ Sentinel API running on http://localhost:{}", port);
    
    axum::serve(listener, app).await?;

    Ok(())
}