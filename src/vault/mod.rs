// Sentinel Vault: SQLite-backed memory system

use rusqlite::{Connection, Result, params};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use dirs::home_dir;

pub mod token_budget;
pub use token_budget::{TokenBudget, estimate_tokens, truncate_to_budget};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryNode {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEdge {
    pub id: i64,
    pub source_id: i64,
    pub target_id: i64,
    pub relation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultSummary {
    pub id: i64,
    pub title: Option<String>,
    pub summary: String,
    pub timestamp: i64,
    pub messages: Option<String>,
}

#[derive(Debug, Clone)]
pub enum VaultMode {
    Pro,
    Lite,
}

pub struct Vault {
    db_path: PathBuf,
    pub mode: VaultMode,
    pub(crate) conn: Connection,
}

pub fn default_db_path() -> PathBuf {
    home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".local/share/sentinx/sentinel/vault.db")
}

impl Vault {
    pub fn new(db_path: PathBuf, mode: VaultMode) -> anyhow::Result<Self> {
        let expanded = shellexpand::tilde(
            db_path.to_str().unwrap_or("")
        ).to_string();
        let path = PathBuf::from(expanded);

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .expect("Failed to create vault directory");
        }

        let conn = Connection::open(&path)?;
        conn.execute("PRAGMA foreign_keys = ON", [])?;

        // Initialize vault schema
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS memory_nodes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                type TEXT NOT NULL DEFAULT 'general',
                description TEXT,
                created_at INTEGER NOT NULL DEFAULT (unixepoch())
            );
            
            CREATE TABLE IF NOT EXISTS memory_edges (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source_id INTEGER REFERENCES memory_nodes(id) ON DELETE CASCADE,
                target_id INTEGER REFERENCES memory_nodes(id) ON DELETE CASCADE,
                relation TEXT NOT NULL,
                weight REAL DEFAULT 1.0
            );
            
            CREATE TABLE IF NOT EXISTS conversation_summaries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT,
                summary TEXT NOT NULL,
                timestamp INTEGER NOT NULL DEFAULT (unixepoch()),
                messages TEXT
            );
            
            CREATE TABLE IF NOT EXISTS preferences (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at INTEGER NOT NULL DEFAULT (unixepoch())
            );
            
            CREATE TABLE IF NOT EXISTS workflows (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT,
                created_at INTEGER NOT NULL DEFAULT (unixepoch())
            );
            
            CREATE TABLE IF NOT EXISTS chat_history (
                id TEXT PRIMARY KEY,
                session_id INTEGER REFERENCES conversation_summaries(id) ON DELETE CASCADE,
                role TEXT NOT NULL CHECK(role IN ('user','assistant')),
                content TEXT NOT NULL,
                timestamp INTEGER NOT NULL DEFAULT (unixepoch())
            );
            
            CREATE INDEX IF NOT EXISTS idx_memory_nodes_name 
                ON memory_nodes(name);
            CREATE INDEX IF NOT EXISTS idx_chat_history_timestamp 
                ON chat_history(timestamp DESC);
        ").expect("Failed to initialize vault schema");

        // Safe migration: Add session_id to chat_history table if it doesn't already exist
        let _ = conn.execute(
            "ALTER TABLE chat_history ADD COLUMN session_id INTEGER REFERENCES conversation_summaries(id) ON DELETE CASCADE",
            [],
        );

        Ok(Vault { db_path: path, mode, conn })
    }

    pub fn save_memory(&self, name: &str, description: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO memory_nodes (name, type, description)
             VALUES (?1, 'manual', ?2)
             ON CONFLICT(name) DO UPDATE SET
               description = excluded.description,
               type = 'manual',
               created_at = unixepoch()",
            params![name, description],
        )?;
        Ok(())
    }

    pub fn save_memory_node(&self, name: &str, ntype: &str, description: &str) -> anyhow::Result<()> {
        if is_noise(name, description) {
            return Ok(());
        }
        self.conn.execute(
            "INSERT INTO memory_nodes (name, type, description)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(name) DO UPDATE SET
               type = excluded.type,
               description = excluded.description,
               created_at = unixepoch()
             WHERE memory_nodes.type != 'manual'",
            params![name, ntype, description],
        )?;
        Ok(())
    }

    pub fn save_memory_edge_by_names(&self, source_name: &str, target_name: &str, relation: &str) -> anyhow::Result<()> {
        if is_noise(source_name, source_name) || is_noise(target_name, target_name) {
            return Ok(());
        }
        self.save_memory_node(source_name, "concept", source_name)?;
        self.save_memory_node(target_name, "concept", target_name)?;
        
        let source_id: i64 = self.conn.query_row(
            "SELECT id FROM memory_nodes WHERE name = ?1",
            params![source_name],
            |row| row.get(0),
        )?;
        let target_id: i64 = self.conn.query_row(
            "SELECT id FROM memory_nodes WHERE name = ?1",
            params![target_name],
            |row| row.get(0),
        )?;
        
        self.conn.execute(
            "INSERT OR IGNORE INTO memory_edges (source_id, target_id, relation) VALUES (?1, ?2, ?3)",
            params![source_id, target_id, relation],
        )?;
        Ok(())
    }

    pub fn search_memory(&self, query: &str) -> Result<Vec<MemoryNode>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, type, description
             FROM memory_nodes
             WHERE name LIKE ?1 OR description LIKE ?1
             ORDER BY created_at DESC
             LIMIT 10"
        )?;
        let query_pattern = format!("%{}%", query);
        let nodes = stmt.query_map(params![query_pattern], |row| {
            Ok(MemoryNode {
                id: row.get(0)?,
                name: row.get(1)?,
                node_type: row.get(2)?,
                description: row.get(3).unwrap_or_default(),
            })
        })?.filter_map(|r| r.ok()).collect();
        Ok(nodes)
    }

    pub fn build_context(&self, prompt: &str, budget: Option<&TokenBudget>) -> Result<String> {
        // Lite mode: zero memory overhead, raw prompt only
        if matches!(self.mode, VaultMode::Lite) {
            return Ok(prompt.to_string());
        }

        let available = budget
            .map(|b| b.available())
            .unwrap_or(1500);
        
        let mut parts: Vec<String> = Vec::new();
        let mut used: u32 = estimate_tokens(prompt);
        
        // Load preferences
        let mut pref_stmt = self.conn.prepare(
            "SELECT key, value FROM preferences LIMIT 20"
        )?;
        let prefs: Vec<String> = pref_stmt
            .query_map([], |r| Ok(format!("{}: {}", 
                r.get::<_,String>(0)?, 
                r.get::<_,String>(1)?)))?
            .filter_map(|r| r.ok())
            .collect();
        if !prefs.is_empty() {
            let pref_text = format!("[Preferences]\n{}", prefs.join("\n"));
            let tokens = estimate_tokens(&pref_text);
            if used + tokens < available {
                parts.push(pref_text);
                used += tokens;
            }
        }
        
        // Search relevant memory nodes
        let mut node_stmt = self.conn.prepare(
            "SELECT name, type, description FROM memory_nodes
             WHERE instr(lower(?1), lower(name)) > 0 OR name LIKE ?2 OR description LIKE ?2
             ORDER BY created_at DESC LIMIT 5"
        )?;
        let pattern = format!("%{}%", 
            prompt.split_whitespace().take(3).collect::<Vec<_>>().join("%"));
        let nodes: Vec<String> = node_stmt
            .query_map(params![prompt, pattern], |r| {
                let name: String = r.get(0)?;
                let ntype: String = r.get(1)?;
                let desc: String = r.get::<_,Option<String>>(2)?
                    .unwrap_or_default();
                Ok(format!("[{}] {}: {}", ntype, name, desc))
            })?
            .filter_map(|r| r.ok())
            .collect();
        println!("BUILD CONTEXT matched nodes: {:?}", nodes);
        if !nodes.is_empty() {
            let node_text = format!("[Memory]\n{}", nodes.join("\n"));
            let tokens = estimate_tokens(&node_text);
            if used + tokens < available {
                parts.push(node_text);
                used += tokens;
            }
        }
        
        // Recent summary
        let summary: Option<String> = self.conn.query_row(
            "SELECT summary FROM conversation_summaries
             ORDER BY timestamp DESC LIMIT 1",
            [],
            |r| r.get(0),
        ).ok();
        if let Some(s) = summary {
            let summary_text = format!("[Previous conversation summary]\n{}", s);
            let tokens = estimate_tokens(&summary_text);
            if used + tokens < available {
                parts.push(summary_text);
            }
        }
        
        if parts.is_empty() {
            return Ok(String::new());
        }
        
        Ok(parts.join("\n\n"))
    }

    pub fn save_summary(&self, title: &str, summary: &str, messages: Option<&str>) -> anyhow::Result<()> {
        let now = chrono::Local::now().timestamp();
        self.conn.execute(
            "INSERT INTO conversation_summaries (title, summary, timestamp, messages) VALUES (?1, ?2, ?3, ?4)",
            params![title, summary, now, messages],
        )?;
        Ok(())
    }

    pub fn list_nodes(&self) -> anyhow::Result<Vec<MemoryNode>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, type, description
             FROM memory_nodes
             ORDER BY created_at DESC"
        )?;
        let nodes = stmt.query_map([], |row| {
            Ok(MemoryNode {
                id: row.get(0)?,
                name: row.get(1)?,
                node_type: row.get(2)?,
                description: row.get::<_, Option<String>>(3)?
                    .unwrap_or_default(),
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
        Ok(nodes)
    }

    pub fn list_edges(&self) -> anyhow::Result<Vec<VaultEdge>> {
        let mut stmt = self.conn.prepare("SELECT id, source_id, target_id, relation FROM memory_edges")?;
        let edges = stmt.query_map([], |row| {
            Ok(VaultEdge {
                id: row.get(0)?,
                source_id: row.get(1)?,
                target_id: row.get(2)?,
                relation: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;

        Ok(edges)
    }

    pub fn list_summaries(&self) -> anyhow::Result<Vec<VaultSummary>> {
        let mut stmt = self.conn.prepare("SELECT id, title, summary, timestamp, messages FROM conversation_summaries")?;
        let summaries = stmt.query_map([], |row| {
            Ok(VaultSummary {
                id: row.get(0)?,
                title: row.get(1)?,
                summary: row.get(2)?,
                timestamp: row.get(3)?,
                messages: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;

        Ok(summaries)
    }

    pub fn get_summary(&self, id: i64) -> anyhow::Result<Option<VaultSummary>> {
        let mut stmt = self.conn.prepare("SELECT id, title, summary, timestamp, messages FROM conversation_summaries WHERE id = ?1")?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(VaultSummary {
                id: row.get(0)?,
                title: row.get(1)?,
                summary: row.get(2)?,
                timestamp: row.get(3)?,
                messages: row.get(4)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn delete_node(&self, id: i64) -> anyhow::Result<()> {
        self.conn.execute("DELETE FROM memory_nodes WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn delete_summary(&self, id: i64) -> anyhow::Result<()> {
        self.conn.execute("DELETE FROM conversation_summaries WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn export_all(&self) -> anyhow::Result<String> {
        let nodes = self.list_nodes()?;
        let json = serde_json::to_string(&nodes)?;
        Ok(json)
    }

    pub fn load_chat_history_for_session(&self, session_id: i64, limit: usize) -> Result<Vec<ChatMessage>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, role, content, timestamp
             FROM chat_history
             WHERE session_id = ?1
             ORDER BY timestamp ASC
             LIMIT ?2"
        )?;
        let msgs = stmt.query_map(params![session_id, limit as i64], |r| {
            Ok(ChatMessage {
                id: r.get(0)?,
                role: r.get(1)?,
                content: r.get(2)?,
                timestamp: r.get(3)?,
            })
        })?.filter_map(|r| r.ok()).collect();
        Ok(msgs)
    }

    pub fn load_chat_history(&self, limit: usize) -> Result<Vec<ChatMessage>> {
        // Load messages from all sessions, ordered by timestamp DESC, limited to most recent N messages
        let mut stmt = self.conn.prepare(
            "SELECT id, role, content, timestamp
             FROM chat_history
             ORDER BY timestamp DESC
             LIMIT ?1"
        )?;
        let mut msgs: Vec<ChatMessage> = stmt.query_map(params![limit as i64], |r| {
            Ok(ChatMessage {
                id: r.get(0)?,
                role: r.get(1)?,
                content: r.get(2)?,
                timestamp: r.get(3)?,
            })
        })?.filter_map(|r| r.ok()).collect();
        
        // Reverse to get chronological order (oldest first)
        msgs.reverse();
        Ok(msgs)
    }

    pub fn clear_chat_history(&self) -> Result<()> {
        self.conn.execute("DELETE FROM chat_history", [])?;
        Ok(())
    }

    pub fn clear_chat_history_for_session(&self, session_id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM chat_history WHERE session_id = ?1", params![session_id])?;
        Ok(())
    }

    pub fn persist_chat_message(
        &self, id: &str, session_id: Option<i64>, role: &str, content: &str, timestamp: i64
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO chat_history (id, session_id, role, content, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5)",
             params![id, session_id, role, content, timestamp],
        )?;
        Ok(())
    }

    pub fn delete_edge(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM memory_edges WHERE id = ?1", params![id])?;
        Ok(())
    }
}

fn is_noise(name: &str, description: &str) -> bool {
    let name = name.trim();
    let desc = description.trim();
    
    // Too short
    if name.len() < 10 { return true; }
    
    // File extensions
    if name.ends_with(".rs") || name.ends_with(".toml") || 
       name.ends_with(".lock") || name.ends_with(".sh") ||
       name.ends_with(".md") || name.ends_with(".json") {
        return true;
    }
    
    // Path-like
    if name.contains('/') || name.contains('\\') { return true; }
    
    // Common greetings
    let greetings = ["hello", "hi", "hey", "thanks", "thank you",
                     "ok", "okay", "yes", "no", "sure", "great"];
    let lower = name.to_lowercase();
    if greetings.iter().any(|g| lower == *g) { return true; }
    
    // Name same as description (duplicate noise)
    if name == desc { return true; }
    
    // Raw LLM reasoning leaked in
    if desc.starts_with("The user is") || 
       desc.starts_with("A formal") ||
       desc.starts_with("The assistant") {
        return true;
    }
    
    false
}
