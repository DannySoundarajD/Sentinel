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

    // FIX 5: Load chat history from SQLite on startup
    let mut initial_history = Vec::new();
    if let Ok(rows) = vault.load_chat_history(50) {
        for (id, role, content, timestamp) in rows {
            initial_history.push(api::ChatMessage { 
                id, 
                role, 
                content, 
                timestamp: timestamp as u64 
            });
        }
    }

    // Initialize Runtime
    let runtime = OllamaRuntime::new(&config.runtime.ollama_host);

    // Initialize Guardian
    let guardian = Guardian::new();

    // Build AppState
    let state = api::AppState {
        vault: Arc::new(Mutex::new(vault)),
        runtime: Arc::new(Mutex::new(runtime)),
        guardian: Arc::new(guardian),
        config: Arc::new(Mutex::new(config)),
        chat_history: Arc::new(Mutex::new(initial_history)),
    };

    // Build router and start server
    let app = api::build_router(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8888").await?;
    println!("✓ Sentinel API running on http://localhost:8888");
    
    axum::serve(listener, app).await?;

    Ok(())
}