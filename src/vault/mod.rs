// Sentinel Vault: SQLite-backed memory system

use rusqlite::{Connection, Result as SqliteResult};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

pub mod token_budget;
pub use token_budget::{TokenBudget, estimate_tokens, truncate_to_budget};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryNode {
    pub id: i64,
    pub key: String,
    pub value: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub enum VaultMode {
    Pro,
    Lite,
}

pub struct Vault {
    db_path: PathBuf,
    mode: VaultMode,
    pub(crate) conn: Connection,
}

impl Vault {
    pub fn new(path: PathBuf, mode: VaultMode) -> anyhow::Result<Self> {
        let conn = Connection::open(&path)?;
        
        // Create tables if not exist
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS memory_nodes (
                id INTEGER PRIMARY KEY,
                key TEXT NOT NULL UNIQUE,
                value TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS memory_edges (id INTEGER PRIMARY KEY);
            CREATE TABLE IF NOT EXISTS conversation_summaries (id INTEGER PRIMARY KEY);
            CREATE TABLE IF NOT EXISTS preferences (id INTEGER PRIMARY KEY);
            CREATE TABLE IF NOT EXISTS workflows (id INTEGER PRIMARY KEY);
            CREATE TABLE IF NOT EXISTS chat_history (
                id TEXT PRIMARY KEY,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp INTEGER NOT NULL
            );",
        )?;

        Ok(Vault { db_path: path, mode, conn })
    }

    pub fn save_memory(&self, key: &str, value: &str) -> anyhow::Result<()> {
        let now = chrono::Local::now().to_rfc3339();
        self.conn.execute(
            "INSERT OR REPLACE INTO memory_nodes (key, value, created_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![key, value, now],
        )?;
        Ok(())
    }

    pub fn search_memory(&self, query: &str) -> anyhow::Result<Vec<MemoryNode>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, key, value, created_at FROM memory_nodes WHERE key LIKE ?1 OR value LIKE ?1"
        )?;
        
        let nodes = stmt.query_map(rusqlite::params![format!("%{}%", query)], |row| {
            Ok(MemoryNode {
                id: row.get(0)?,
                key: row.get(1)?,
                value: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?
        .collect::<SqliteResult<Vec<_>>>()?;

        Ok(nodes)
    }

    pub fn build_context(&self, prompt: &str, budget: Option<&TokenBudget>) -> anyhow::Result<String> {
        // Lite mode: skip all memory context, just return raw prompt
        match self.mode {
            VaultMode::Lite => {
                return Ok(prompt.to_string());
            }
            VaultMode::Pro => {}
        }

        // Pro mode: assemble context with budget priority
        let budget = budget.cloned().unwrap_or_else(|| TokenBudget::new(2048));
        let mut available = budget.available();

        let mut context_parts = Vec::new();

        // 1. Always include the prompt
        context_parts.push(format!("[Current Prompt]\n{}", prompt));
        available = available.saturating_sub(estimate_tokens(prompt));

        // 2. Add preferences (small, almost always fits)
        if available > 100 {
            context_parts.push("[User Preferences]\nAI Assistant optimized for efficiency.".to_string());
            available = available.saturating_sub(100);
        }

        // 3. Add recent conversation history (50% of remaining budget)
        if available > 200 {
            let history_budget = (available as f32 * 0.5) as u32;
            let history = format!(
                "[Recent Context]\nThis is the start of a new conversation."
            );
            let truncated_history = truncate_to_budget(&history, history_budget);
            context_parts.push(truncated_history.clone());
            available = available.saturating_sub(estimate_tokens(&truncated_history));
        }

        // 4. Add memory nodes (30% of remaining)
        if available > 100 {
            let nodes_budget = (available as f32 * 0.3) as u32;
            if let Ok(nodes) = self.search_memory(prompt) {
                if !nodes.is_empty() {
                    let top_3 = nodes.iter().take(3).collect::<Vec<_>>();
                    let mut nodes_text = "[Related Knowledge]\n".to_string();
                    for node in top_3 {
                        let node_budget = nodes_budget / 3;
                        let truncated = truncate_to_budget(&node.value, node_budget);
                        nodes_text.push_str(&format!("• {} - {}\n", node.key, truncated));
                    }
                    context_parts.push(nodes_text.clone());
                    #[allow(unused_assignments)]
                    {
                        available = available.saturating_sub(estimate_tokens(&nodes_text));
                    }
                }
            }
        }

        Ok(context_parts.join("\n\n"))
    }

    pub fn save_summary(&self, _title: &str, _summary: &str) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn list_nodes(&self) -> anyhow::Result<Vec<MemoryNode>> {
        let mut stmt = self.conn.prepare("SELECT id, key, value, created_at FROM memory_nodes")?;
        let nodes = stmt.query_map([], |row| {
            Ok(MemoryNode {
                id: row.get(0)?,
                key: row.get(1)?,
                value: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?
        .collect::<SqliteResult<Vec<_>>>()?;

        Ok(nodes)
    }

    pub fn delete_node(&self, id: i64) -> anyhow::Result<()> {
        self.conn.execute("DELETE FROM memory_nodes WHERE id = ?1", rusqlite::params![id])?;
        Ok(())
    }

    pub fn export_all(&self) -> anyhow::Result<String> {
        let nodes = self.list_nodes()?;
        let json = serde_json::to_string(&nodes)?;
        Ok(json)
    }

    // FIX 5: Load chat history from SQLite
    pub fn load_chat_history(&self, limit: i64) -> anyhow::Result<Vec<(String, String, String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, role, content, timestamp FROM chat_history ORDER BY timestamp DESC LIMIT ?1"
        )?;
        let history = stmt.query_map([limit], |row| {
            Ok((
                row.get(0)?,  // id
                row.get(1)?,  // role
                row.get(2)?,  // content
                row.get(3)?,  // timestamp
            ))
        })?;
        
        let mut results = Vec::new();
        for msg in history {
            results.push(msg?);
        }
        results.reverse(); // Reverse to get chronological order
        Ok(results)
    }

    // FIX 5: Clear chat history
    pub fn clear_chat_history(&self) -> anyhow::Result<()> {
        self.conn.execute("DELETE FROM chat_history", [])?;
        Ok(())
    }

    // FIX 5: Save chat message
    pub fn save_chat_message(&self, id: &str, role: &str, content: &str, timestamp: i64) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO chat_history (id, role, content, timestamp) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, role, content, timestamp],
        )?;
        Ok(())
    }
}
