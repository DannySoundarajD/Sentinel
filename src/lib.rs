#![warn(clippy::all)]
#![allow(dead_code, unused_variables, unused_imports)]

use serde::{Deserialize, Serialize};

// Sentinel Library
pub mod agent;
pub mod api;
pub mod bridge;
pub mod config;
pub mod daemon;
pub mod guardian;
pub mod hotkey;
pub mod notifications;
pub mod providers;
pub mod runtime;
pub mod security;
pub mod skillforge;
pub mod skills;
pub mod tools;
pub mod tray;
pub mod vault;

// Re-export key types
pub use config::Config;
pub use vault::Vault;
pub use runtime::OllamaRuntime;
pub use agent::Agent;