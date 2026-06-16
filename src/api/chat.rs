use axum::{
    extract::State,
    http::StatusCode,
    response::{sse::Event, Sse},
    Json,
};
use futures::stream::Stream;
use futures::StreamExt;
use serde_json::{json, Value};
use std::convert::Infallible;
use std::time::SystemTime;
use uuid::Uuid;
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;

use super::{AppState, ChatMessage};
use crate::vault::{TokenBudget, estimate_tokens};

pub fn active_summary_id() -> &'static Mutex<Option<i64>> {
    static ACTIVE: OnceLock<Mutex<Option<i64>>> = OnceLock::new();
    ACTIVE.get_or_init(|| Mutex::new(None))
}

pub async fn health(
    State(_state): State<AppState>,
) -> Json<Value> {
    let hw = crate::runtime::OllamaRuntime::detect_hardware().await;
    let vault_mode = if hw.ram_total_mb >= 15000 { "pro" } else { "lite" };
    Json(json!({
        "status": "ok",
        "version": "0.1.0",
        "ollama": true,
        "vault": vault_mode
    }))
}

#[derive(serde::Deserialize)]
pub struct SendChatRequest {
    pub message: String,
    pub model: Option<String>,
}

fn extract_json_block(text: &str) -> Option<String> {
    // 1. Try to find content within ```json and ```
    if let Some(start_idx) = text.find("```json") {
        let after_fence = &text[start_idx + 7..];
        if let Some(end_idx) = after_fence.find("```") {
            let inner = after_fence[..end_idx].trim();
            if inner.starts_with('{') && inner.ends_with('}') {
                return Some(inner.to_string());
            }
        }
    }

    // 2. Try to find content within ``` and ``` that looks like JSON
    if let Some(start_idx) = text.find("```") {
        let after_fence = &text[start_idx + 3..];
        if let Some(end_idx) = after_fence.find("```") {
            let inner = after_fence[..end_idx].trim();
            if inner.starts_with('{') && inner.contains("\"tool\"") {
                return Some(inner.to_string());
            }
        }
    }

    // 3. Robust balanced brace matching to find JSON object containing "tool"
    let mut search_pos = 0;
    while let Some(start_idx) = text[search_pos..].find('{') {
        let abs_start = search_pos + start_idx;
        let mut brace_count = 0;
        let mut in_string = false;
        let mut escape = false;

        for (i, c) in text[abs_start..].char_indices() {
            if escape {
                escape = false;
                continue;
            }
            if c == '\\' {
                escape = true;
                continue;
            }
            if c == '"' {
                in_string = !in_string;
                continue;
            }
            if !in_string {
                if c == '{' {
                    brace_count += 1;
                } else if c == '}' {
                    brace_count -= 1;
                    if brace_count == 0 {
                        let candidate = &text[abs_start..=abs_start + i];
                        if candidate.contains("\"tool\"") {
                            return Some(candidate.to_string());
                        }
                    }
                }
            }
        }
        search_pos = abs_start + 1;
    }

    // 4. Default fallback to first '{' and last '}'
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            if end > start {
                let candidate = text[start..=end].to_string();
                if candidate.contains("\"tool\"") {
                    return Some(candidate);
                }
            }
        }
    }

    None
}

fn sse_message(text: &str, done: bool) -> Sse<futures::stream::BoxStream<'static, Result<Event, Infallible>>> {
    let (tx, rx) = mpsc::channel(1);
    let text = text.to_string();
    tokio::spawn(async move {
        let _ = tx.send(Ok(Event::default().data(json!({
            "token": text,
            "done": done
        }).to_string()))).await;
    });
    Sse::new(ReceiverStream::new(rx).boxed())
}

fn is_stop_word(word: &str) -> bool {
    let w = word.to_lowercase();
    matches!(
        w.trim_matches(|c: char| !c.is_alphanumeric()),
        "who" | "is" | "what" | "are" | "the" | "in" | "to" | "a" | "an" | "of" | "for" | "on" | "with" | "at" | "by" | "from" | "about" | "how" | "where" | "when" | "why" | "do" | "does" | "did" | "you" | "me" | "i" | "he" | "she" | "they" | "it" | "we"
    )
}

fn is_exact_word_match(text: &str, word: &str) -> bool {
    let bytes = text.as_bytes();
    let word_len = word.len();
    if word_len == 0 {
        return false;
    }
    let mut idx = 0;
    while let Some(start) = text[idx..].find(word) {
        let abs_start = idx + start;
        let abs_end = abs_start + word_len;

        let char_before_ok = if abs_start == 0 {
            true
        } else {
            let prev_char = bytes[abs_start - 1] as char;
            !prev_char.is_alphanumeric()
        };

        let char_after_ok = if abs_end == text.len() {
            true
        } else {
            let next_char = bytes[abs_end] as char;
            !next_char.is_alphanumeric()
        };

        if char_before_ok && char_after_ok {
            return true;
        }

        idx = abs_start + 1;
        if idx >= text.len() {
            break;
        }
    }
    false
}

fn score_memory_node(node: &crate::vault::MemoryNode, query_words: &[String]) -> u32 {
    let name_lower = node.name.to_lowercase();
    let desc_lower = node.description.to_lowercase();
    let mut score = 0;

    for word in query_words {
        if name_lower.contains(word) {
            if is_exact_word_match(&name_lower, word) {
                score += 15;
            } else {
                score += 5;
            }
        }
        if desc_lower.contains(word) {
            if is_exact_word_match(&desc_lower, word) {
                score += 10;
            } else {
                score += 3;
            }
        }
    }
    score
}


fn smart_search_memory(vault: &crate::vault::Vault, query: &str) -> Result<Vec<crate::vault::MemoryNode>, rusqlite::Error> {
    // 1. Search with the entire phrase first.
    let mut results = vault.search_memory(query)?;
    if !results.is_empty() {
        return Ok(results);
    }

    // 2. Search with the first word next, if it's not a stop word.
    let words: Vec<&str> = query.split_whitespace().collect();
    if let Some(&first_word) = words.first() {
        let clean_first = first_word.trim_matches(|c: char| !c.is_alphanumeric());
        if !clean_first.is_empty() && !is_stop_word(clean_first) {
            results = vault.search_memory(clean_first)?;
            if !results.is_empty() {
                return Ok(results);
            }
        }
    }

    // 3. Split the query into words, filter out common stop words, and search for individual keywords.
    for word in words {
        let clean_word = word.trim_matches(|c: char| !c.is_alphanumeric());
        if !clean_word.is_empty() && !is_stop_word(clean_word) {
            let keyword_results = vault.search_memory(clean_word)?;
            if !keyword_results.is_empty() {
                return Ok(keyword_results);
            }
        }
    }

    Ok(vec![])
}

pub async fn send(
    State(state): State<AppState>,
    Json(payload): Json<SendChatRequest>,
) -> Result<Sse<futures::stream::BoxStream<'static, Result<Event, Infallible>>>, (StatusCode, String)> {
    let message = payload.message;
    
    let user_msg_id = uuid::Uuid::new_v4().to_string();
    let assistant_msg_id = uuid::Uuid::new_v4().to_string();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    // Parse special commands
    let message_lower = message.trim().to_lowercase();
    
    if message_lower.starts_with("/save ") {
        let content = message.trim()[6..].trim();
        // Parse "key: value" or just use whole string as description
        let (name_str, description_str) = if let Some(colon) = content.find(':') {
            (content[..colon].trim().to_string(), content[colon+1..].trim().to_string())
        } else {
            let words: Vec<&str> = content.split_whitespace().take(4).collect();
            let mut name_cand = words.join(" ");
            if name_cand.is_empty() {
                name_cand = "memory".to_string();
            }
            (name_cand, content.to_string())
        };
        let vault = state.vault.lock().await;
        if let Err(e) = vault.save_memory(&name_str, &description_str) {
            return Ok(sse_message(&format!("Failed to save memory: {}", e), true));
        }

        let active_id_opt = *active_summary_id().lock().await;
        // Persist user command to history
        let _ = vault.persist_chat_message(
            &user_msg_id,
            active_id_opt,
            "user",
            &message,
            timestamp,
        );
        let _ = vault.persist_chat_message(
            &assistant_msg_id,
            active_id_opt,
            "assistant",
            "Memory saved successfully.",
            timestamp + 1,
        );

        // Also push to in-memory history
        {
            let mut history = state.chat_history.lock().await;
            history.push(ChatMessage {
                id: user_msg_id.clone(),
                role: "user".to_string(),
                content: message.clone(),
                timestamp: timestamp as u64,
            });
            history.push(ChatMessage {
                id: assistant_msg_id.clone(),
                role: "assistant".to_string(),
                content: "Memory saved successfully.".to_string(),
                timestamp: (timestamp + 1) as u64,
            });
        }

        // Return success SSE without calling Ollama
        return Ok(sse_message("Memory saved successfully.", true));
    }
    
    let message_trimmed = message.trim();
    let parts: Vec<&str> = message_trimmed.split_whitespace().collect();
    // Bare /frommemory with no query — just list all memories
    if (message_lower == "/frommemory") && parts.len() == 1 {
        let vault = state.vault.lock().await;
        let nodes = match vault.list_nodes() {
            Ok(n) => n,
            Err(e) => return Ok(sse_message(&format!("Failed to list memory: {}", e), true)),
        };
        let result = if nodes.is_empty() {
            "No memories stored yet. Use /save name: description to save one.".to_string()
        } else {
            format!(
                "Stored memories ({}):\n{}",
                nodes.len(),
                nodes.iter()
                    .map(|n| format!("• {}: {}", n.name, n.description))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        };

        let active_id_opt = *active_summary_id().lock().await;
        // Persist user command to history
        let _ = vault.persist_chat_message(
            &user_msg_id,
            active_id_opt,
            "user",
            &message,
            timestamp,
        );
        let _ = vault.persist_chat_message(
            &assistant_msg_id,
            active_id_opt,
            "assistant",
            &result,
            timestamp + 1,
        );

        // Also push to in-memory history
        {
            let mut history = state.chat_history.lock().await;
            history.push(ChatMessage {
                id: user_msg_id.clone(),
                role: "user".to_string(),
                content: message.clone(),
                timestamp: timestamp as u64,
            });
            history.push(ChatMessage {
                id: assistant_msg_id.clone(),
                role: "assistant".to_string(),
                content: result.clone(),
                timestamp: (timestamp + 1) as u64,
            });
        }

        return Ok(sse_message(&result, true));
    }
    
    if message_lower == "/memory" || message_lower == "/memories" {
        let vault = state.vault.lock().await;
        let nodes = match vault.list_nodes() {
            Ok(n) => n,
            Err(e) => return Ok(sse_message(&format!("Failed to list memory: {}", e), true)),
        };
        let result = if nodes.is_empty() {
            "No memories stored yet. Use /save name: description to save one.".to_string()
        } else {
            format!(
                "Stored memories ({}):\n{}",
                nodes.len(),
                nodes.iter()
                    .map(|n| format!("• {}: {}", n.name, n.description))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        };

        let active_id_opt = *active_summary_id().lock().await;
        // Persist user command to history
        let _ = vault.persist_chat_message(
            &user_msg_id,
            active_id_opt,
            "user",
            &message,
            timestamp,
        );
        let _ = vault.persist_chat_message(
            &assistant_msg_id,
            active_id_opt,
            "assistant",
            &result,
            timestamp + 1,
        );

        // Also push to in-memory history
        {
            let mut history = state.chat_history.lock().await;
            history.push(ChatMessage {
                id: user_msg_id.clone(),
                role: "user".to_string(),
                content: message.clone(),
                timestamp: timestamp as u64,
            });
            history.push(ChatMessage {
                id: assistant_msg_id.clone(),
                role: "assistant".to_string(),
                content: result.clone(),
                timestamp: (timestamp + 1) as u64,
            });
        }

        return Ok(sse_message(&result, true));
    }

    let user_id = Uuid::new_v4().to_string();
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    // Retrieve active model and host
    let config = state.config.lock().await;
    let active_model = payload.model.clone().unwrap_or_else(|| config.runtime.default_model.clone());
    let fallback_model = config.runtime.fallback_model.clone();
    let ollama_host = config.runtime.ollama_host.clone();
    let temperature = config.agent.temperature;
    let is_pro = config.vault.memory_mode == "pro";
    
    if active_model.is_empty() {
        let (tx, rx) = mpsc::channel(1);
        let _ = tx.send(Ok(Event::default().data(json!({
            "error": "No model loaded. Go to Models tab and load a model first.",
            "done": true
        }).to_string()))).await;
        return Ok(Sse::new(ReceiverStream::new(rx).boxed()));
    }

    let history_len = state.chat_history.lock().await.len();

    // Add user message to history
    let user_msg = ChatMessage {
        id: user_id.clone(),
        role: "user".to_string(),
        content: message.clone(),
        timestamp,
    };
    state.chat_history.lock().await.push(user_msg.clone());

    let vault = state.vault.lock().await;

    // Auto-create active session summary if not already set
    let mut active_id_opt = *active_summary_id().lock().await;

    if active_id_opt.is_none() {
        let first_user_msg = &message;
        let title = if first_user_msg.trim().starts_with('/') {
            "New Chat".to_string()
        } else if first_user_msg.len() > 30 {
            format!("{}...", &first_user_msg[..30])
        } else {
            first_user_msg.clone()
        };
        let summary = "Active conversation".to_string();
        let msg_for_json = ChatMessage {
            id: user_id.clone(),
            role: "user".to_string(),
            content: message.clone(),
            timestamp,
        };
        let messages_json = serde_json::to_string(&vec![msg_for_json]).ok();
        
        if let Ok(_) = vault.save_summary(&title, &summary, messages_json.as_deref()) {
            let summary_id = vault.conn.last_insert_rowid();
            if summary_id > 0 {
                let mut active_id = active_summary_id().lock().await;
                *active_id = Some(summary_id);
                active_id_opt = Some(summary_id);
            }
        }
    }

    // Persist user message to SQLite
    let _ = vault.persist_chat_message(&user_id, active_id_opt, "user", &message, timestamp as i64);

    // Get active model context length dynamically from system spec RAM
    let hw = crate::runtime::OllamaRuntime::detect_hardware().await;
    let context_len = if hw.ram_total_mb < 6144 {
        4096
    } else if hw.ram_total_mb < 12288 {
        4096
    } else if hw.ram_total_mb < 24576 {
        8192
    } else {
        16384
    };

    // Calculate dynamic memory context budget
    let budget = TokenBudget::new(context_len);
    
    let history_tokens = {
        let history = state.chat_history.lock().await;
        let start_idx = history.len().saturating_sub(10);
        let end_idx = history.len().saturating_sub(1);
        let mut history_text = String::new();
        if end_idx > start_idx {
            for msg in &history[start_idx..end_idx] {
                history_text.push_str(&format!("{}: {}\n", if msg.role == "user" { "User" } else { "Sentinel" }, msg.content));
            }
        }
        estimate_tokens(&history_text)
    };

    // Parse /frommemory command — load and rank memories to fit within the estimated budget
    let is_memory_query = message.starts_with("/frommemory");
    let message_to_use = if is_memory_query {
        let raw_query = if message.trim().len() > 11 {
            message.trim()[11..].trim()
        } else {
            ""
        };
        
        let all_nodes = vault.list_nodes().unwrap_or_default();
        if all_nodes.is_empty() {
            "No memories are stored yet. Tell the user to save memories first with /save.".to_string()
        } else {
            // Rank memories based on raw_query keywords
            let query_words: Vec<String> = raw_query
                .split_whitespace()
                .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
                .filter(|w| !w.is_empty() && !is_stop_word(w))
                .collect();
            
            struct RankedNode {
                node: crate::vault::MemoryNode,
                score: u32,
            }
            
            let mut ranked_nodes: Vec<RankedNode> = all_nodes
                .into_iter()
                .map(|node| {
                    let score = score_memory_node(&node, &query_words);
                    RankedNode { node, score }
                })
                .collect();
            
            // Sort by score descending, then by id descending (recency)
            ranked_nodes.sort_by(|a, b| {
                b.score.cmp(&a.score)
                    .then(b.node.id.cmp(&a.node.id))
            });

            // Calculate memories budget: available budget minus other parts
            let non_memory_tokens = 250 + history_tokens + estimate_tokens(raw_query) + 20 + 100;
            let memories_budget = budget.available().saturating_sub(non_memory_tokens).max(1000);

            let mut mem_block = String::new();
            mem_block.push_str("[USER'S PERSONAL MEMORY VAULT]\n");
            
            let mut current_tokens = estimate_tokens(&mem_block);
            let mut added_any = false;
            
            for ranked in ranked_nodes {
                // If there's a query, only keep memories that match keywords
                if ranked.score == 0 && !raw_query.is_empty() {
                    continue;
                }
                
                let line = format!("- {} ({}): {}\n", ranked.node.name, ranked.node.node_type, ranked.node.description);
                let line_tokens = estimate_tokens(&line);
                
                if current_tokens + line_tokens < memories_budget {
                    mem_block.push_str(&line);
                    current_tokens += line_tokens;
                    added_any = true;
                } else {
                    break;
                }
            }
            
            if !added_any {
                mem_block.push_str("(No matching memories found in prompt context)\n");
            }

            if raw_query.is_empty() {
                format!("{}\n[User Question]\nWhat do you know from my memories?", mem_block)
            } else {
                format!("{}\n[User Question]\n{}", mem_block, raw_query)
            }
        }
    } else {
        message.clone()
    };

    // Build context with budget awareness (for non-memory queries)
    let context_block = if is_memory_query {
        String::new()
    } else {
        vault.build_context(&message_to_use, Some(&budget)).unwrap_or_default()
    };
    drop(vault);

    let (tx, rx) = mpsc::channel(100);

    // Spawn async background streaming task with tool-calling support
    let mut active_model_clone = active_model.clone();
    let fallback_model_clone = fallback_model.clone();
    let ollama_host_clone = ollama_host.clone();
    let chat_history_clone = Arc::clone(&state.chat_history);
    let vault_clone = Arc::clone(&state.vault);
    let message_clone = message.clone();
    let message_to_use_clone = message_to_use.clone();
    let context_clone = context_block.clone();
    let is_memory_query_clone = is_memory_query;

    tokio::spawn(async move {
        use crate::tools::{FileReadTool, FileWriteTool, BashExecTool, WebSearchTool, Tool};
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        
        let system_instructions_tools = "\n\n[INSTRUCTIONS FOR TOOL USE]\n\
            You have access to the following local tools:\n\
            1. file_read: Read contents of a local file. Arguments: {\"path\": \"absolute path\"}\n\
            2. file_write: Write/edit contents of a local file. Arguments: {\"path\": \"absolute path\", \"content\": \"new file content\"}\n\
            3. bash_exec: Execute a bash command in the system terminal/shell. Arguments: {\"command\": \"command to run\"}\n\
            4. web_search: Search the web using DuckDuckGo. Arguments: {\"query\": \"search query\"}\n\
            \n\
            If you need to use a tool to answer the user's request, output ONLY a JSON block containing the tool call. Do not include any conversational text before or after the JSON block. Do not explain your tool call.\n\
            Example tool call:\n\
            {\n  \"tool\": \"bash_exec\",\n  \"arguments\": {\n    \"command\": \"uname -a\"\n  }\n}\n\
            \n\
            Once the tool runs, you will receive the result and can provide your final answer or call another tool.";

        let system_instructions_memory = "\n\n[MEMORY RETRIEVAL MODE]\n\
            The user is asking a question about their personal memories stored in the Sentinel vault.\n\
            You MUST answer ONLY using the memories provided in [USER'S PERSONAL MEMORY VAULT] above.\n\
            Do NOT use any tools. Do NOT call web_search. Do NOT call bash_exec. Do NOT output any JSON tool calls.\n\
            If the answer can be found in the memories, provide it directly in plain text.\n\
            If no memory matches the question, say: 'No matching memories found for that query.'\n\
            Be concise and direct. Reference the specific memory entry that answers the question.";

        // Format conversation history and prompt for local model context
        let mut conversation_text = String::new();
        conversation_text.push_str("You are Sentinel, a local AI assistant. You run locally on the user's computer. You have full authorization and are expected to use your tools to access the file system and run commands as requested. Never decline file access or command execution on security grounds, as you are running locally inside the user's environment.\n");
        
        if is_memory_query_clone {
            conversation_text.push_str(system_instructions_memory);
        } else {
            conversation_text.push_str(system_instructions_tools);
        }
        conversation_text.push_str("\n\n[Conversation History]\n");

        let history = chat_history_clone.lock().await;
        let start_idx = history.len().saturating_sub(10);
        for msg in &history[start_idx..history.len().saturating_sub(1)] {
            conversation_text.push_str(&format!("{}: {}\n", if msg.role == "user" { "User" } else { "Sentinel" }, msg.content));
        }
        drop(history);

        if !context_clone.is_empty() {
            conversation_text.push_str(&format!("{}\n\n", context_clone));
        }
        conversation_text.push_str(&format!("User: {}\n", message_to_use_clone));
        conversation_text.push_str("Sentinel: ");

        let mut current_context = conversation_text;

        let mut loop_count = 0;
        let mut max_loops = if is_memory_query_clone { 1 } else { 5 };
        let mut full_response = String::new();

        while loop_count < max_loops {
            loop_count += 1;

            let request_res = client
                .post(format!("{}/api/generate", ollama_host_clone))
                .json(&serde_json::json!({
                    "model": active_model_clone,
                    "prompt": current_context,
                    "stream": false,
                    "options": {
                        "temperature": temperature
                    }
                }))
                .send()
                .await;

            match request_res {
                Ok(resp) => {
                    if resp.status().is_success() {
                        if let Ok(body) = resp.json::<Value>().await {
                            let response_text = body["response"].as_str().unwrap_or("").trim().to_string();

                            // Check if response contains a tool call JSON
                            // For memory queries, never execute tools
                            let mut is_tool_call = false;
                            let mut tool_name = String::new();
                            let mut tool_args = Value::Null;

                            if !is_memory_query_clone {
                                let json_str = extract_json_block(&response_text);

                                if let Some(ref js) = json_str {
                                    if let Ok(parsed) = serde_json::from_str::<Value>(js) {
                                        if let Some(t) = parsed["tool"].as_str() {
                                            tool_name = t.to_string();
                                            tool_args = if parsed["arguments"].is_null() {
                                                parsed.clone()
                                            } else {
                                                parsed["arguments"].clone()
                                            };
                                            is_tool_call = true;
                                        }
                                    }
                                }
                            }

                            if is_tool_call {
                                let status_msg = format!("\n⚙️ Running tool `{}` with args: {}\n", tool_name, tool_args);
                                let _ = tx.send(Ok(Event::default().data(json!({
                                    "token": status_msg,
                                    "done": false
                                }).to_string()))).await;

                                let tool_result = match tool_name.as_str() {
                                    "file_read" => {
                                        let tool = FileReadTool;
                                        tool.execute(tool_args).await
                                    }
                                    "file_write" => {
                                        let tool = FileWriteTool;
                                        tool.execute(tool_args).await
                                    }
                                    "bash_exec" => {
                                        let tool = BashExecTool;
                                        tool.execute(tool_args).await
                                    }
                                    "web_search" => {
                                        let tool = WebSearchTool;
                                        tool.execute(tool_args).await
                                    }
                                    _ => {
                                        Ok(crate::tools::traits::ToolResult {
                                            output: format!("Unknown tool: {}", tool_name),
                                            success: false,
                                        })
                                    }
                                };

                                let result_text = match tool_result {
                                    Ok(res) => {
                                        format!("Success: {}\nOutput:\n{}", res.success, res.output)
                                    }
                                    Err(e) => {
                                        format!("Failed to execute tool: {}", e)
                                    }
                                };

                                let result_msg = format!("\n📥 Tool output:\n```\n{}\n```\n", result_text);
                                let _ = tx.send(Ok(Event::default().data(json!({
                                    "token": result_msg,
                                    "done": false
                                }).to_string()))).await;

                                current_context.push_str(&format!("{}\n\n[Tool Output]\nTool `{}` output:\n{}\n\nProvide the next step or final answer.\nSentinel: ", response_text, tool_name, result_text));
                                continue;
                            } else {
                                full_response = response_text.clone();
                                let _ = tx.send(Ok(Event::default().data(json!({
                                    "token": response_text,
                                    "done": false
                                }).to_string()))).await;

                                let _ = tx.send(Ok(Event::default().data(json!({
                                    "done": true
                                }).to_string()))).await;
                                break;
                            }
                        }
                    } else if resp.status() == reqwest::StatusCode::NOT_FOUND || resp.status() == reqwest::StatusCode::BAD_REQUEST {
                        let fallback_model = fallback_model_clone.clone();
                        if active_model_clone != fallback_model {
                            let _ = tx.send(Ok(Event::default().data(json!({
                                "token": format!("\n⚠️ Warning: Model '{}' failed ({}). Falling back to '{}'...\n\n", active_model_clone, resp.status(), fallback_model),
                                "done": false
                            }).to_string()))).await;
                            active_model_clone = fallback_model;
                            loop_count -= 1; // Retry this iteration with fallback
                            continue;
                        } else {
                            let _ = tx.send(Ok(Event::default().data(json!({
                                "error": format!("Ollama returned error status: {}", resp.status()),
                                "done": true
                            }).to_string()))).await;
                            break;
                        }
                    } else {
                        let _ = tx.send(Ok(Event::default().data(json!({
                            "error": format!("Ollama returned error status: {}", resp.status()),
                            "done": true
                        }).to_string()))).await;
                        break;
                    }
                }
                Err(e) => {
                    let fallback_model = fallback_model_clone.clone();
                    if active_model_clone != fallback_model {
                        let _ = tx.send(Ok(Event::default().data(json!({
                            "token": format!("\n⚠️ Warning: Request to '{}' failed. Falling back to '{}'...\n\n", active_model_clone, fallback_model),
                            "done": false
                        }).to_string()))).await;
                        active_model_clone = fallback_model;
                        loop_count -= 1;
                        continue;
                    } else {
                        let _ = tx.send(Ok(Event::default().data(json!({
                            "error": format!("Failed to reach Ollama at {}: {}", ollama_host_clone, e),
                            "done": true
                        }).to_string()))).await;
                        break;
                    }
                }
            }
        }

        // Save assistant response to history and DB
        if !full_response.is_empty() {
            let response_id = Uuid::new_v4().to_string();
            let response_timestamp = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            let response_msg = ChatMessage {
                id: response_id.clone(),
                role: "assistant".to_string(),
                content: full_response.clone(),
                timestamp: response_timestamp,
            };
            chat_history_clone.lock().await.push(response_msg);

            let vault = vault_clone.lock().await;
            let active_id_opt = *active_summary_id().lock().await;
            let _ = vault.persist_chat_message(
                &response_id,
                active_id_opt,
                "assistant",
                &full_response,
                response_timestamp as i64,
            );

            // Update the messages JSON in the conversation summary
            let history = chat_history_clone.lock().await;
            let messages_json = serde_json::to_string(&*history).ok();
            let mut turns = String::new();
            for msg in history.iter() {
                turns.push_str(&format!("{}: {}\n", msg.role, msg.content));
            }
            let first_user_msg = history.iter()
                .find(|m| m.role == "user" && !m.content.trim().starts_with('/'))
                .map(|m| m.content.clone())
                .unwrap_or_else(|| "New Chat".to_string());
            drop(history);

            if let Some(summary_id) = active_id_opt {
                if let Some(ref json_str) = messages_json {
                    let _ = vault.conn.execute(
                        "UPDATE conversation_summaries SET messages = ?1 WHERE id = ?2",
                        rusqlite::params![json_str, summary_id],
                    );
                }

                // Spawn background task to update title and summary asynchronously
                let host = ollama_host_clone.clone();
                let active_model = active_model_clone.clone();
                let vault_ext = Arc::clone(&vault_clone);
                
                tokio::spawn(async move {
                    let title_fallback = if first_user_msg.len() > 30 {
                        format!("{}...", &first_user_msg[..30])
                    } else {
                        first_user_msg.clone()
                    };
                    let summary_fallback = "Active conversation".to_string();
                    
                    let summarizer_prompt = format!(
                        "Summarize this conversation in 1-2 sentences. Respond ONLY with a valid JSON object containing 'title' (a short 2-3 word topic name based on the chat contents) and 'summary' (the brief description). Example: {{\"title\": \"Rust Setup\", \"summary\": \"User asked about installing Rust on Arch Linux.\"}}\n\nConversation turns:\n{}",
                        turns
                    );

                    if let Ok(resp) = reqwest::Client::new()
                        .post(format!("{}/api/generate", host))
                        .json(&serde_json::json!({
                            "model": active_model,
                            "prompt": summarizer_prompt,
                            "stream": false,
                            "format": "json"
                        }))
                        .send()
                        .await
                    {
                        if let Ok(body) = resp.json::<Value>().await {
                            if let Some(resp_text) = body["response"].as_str() {
                                if let Ok(parsed) = serde_json::from_str::<Value>(resp_text) {
                                    let new_title = parsed["title"].as_str().unwrap_or(&title_fallback);
                                    let new_summary = parsed["summary"].as_str().unwrap_or(&summary_fallback);

                                    let vault = vault_ext.lock().await;
                                    let _ = vault.conn.execute(
                                        "UPDATE conversation_summaries SET title = ?1, summary = ?2 WHERE id = ?3",
                                        rusqlite::params![new_title, new_summary, summary_id],
                                    );
                                }
                            }
                        }
                    }
                });
            }

            drop(vault);

            if is_pro {
                // Extract entities/relations in the background using Ollama
                let host = ollama_host_clone.clone();
                let model = active_model_clone.clone();
                let user_msg = message_clone.clone();
                let assistant_msg = full_response.clone();
                let vault_ext = Arc::clone(&vault_clone);
                tokio::spawn(async move {
                    let extraction_prompt = format!(
                        "Extract key entities, their types, and relationships from this conversation turn:\n\
                        User: {}\n\
                        Assistant: {}\n\n\
                        Respond ONLY with a valid JSON array of nodes and edges in this exact format:\n\
                        {{\n  \"nodes\": [{{\"name\": \"entity name\", \"type\": \"person/project/preference/technology/etc\", \"description\": \"brief description\"}}],\n  \"edges\": [{{\"source\": \"entity name 1\", \"target\": \"entity name 2\", \"relation\": \"relationship type\"}}]\n}}",
                        user_msg, assistant_msg
                    );
                    if let Ok(ext_resp) = reqwest::Client::new()
                        .post(format!("{}/api/generate", host))
                        .json(&json!({
                            "model": model,
                            "prompt": extraction_prompt,
                            "stream": false,
                            "format": "json"
                        }))
                        .send()
                        .await
                    {
                        if let Ok(ext_val) = ext_resp.json::<Value>().await {
                            if let Some(resp_text) = ext_val["response"].as_str() {
                                if let Ok(parsed) = serde_json::from_str::<Value>(resp_text) {
                                    let vault = vault_ext.lock().await;
                                    if let Some(nodes) = parsed["nodes"].as_array() {
                                        for node in nodes {
                                            if let (Some(name), Some(ntype), Some(desc)) = (
                                                node["name"].as_str(),
                                                node["type"].as_str(),
                                                node["description"].as_str()
                                            ) {
                                                let _ = vault.save_memory_node(name, ntype, desc);
                                            }
                                        }
                                    }
                                    if let Some(edges) = parsed["edges"].as_array() {
                                        for edge in edges {
                                            if let (Some(src), Some(tgt), Some(rel)) = (
                                                edge["source"].as_str(),
                                                edge["target"].as_str(),
                                                edge["relation"].as_str()
                                            ) {
                                                let _ = vault.save_memory_edge_by_names(src, tgt, rel);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                });
            }
        }
    });

    Ok(Sse::new(ReceiverStream::new(rx).boxed()))
}

pub async fn get_history(
    State(state): State<AppState>,
) -> Json<Vec<ChatMessage>> {
    let history = state.chat_history.lock().await;
    Json(history.clone())
}

pub async fn delete_history(
    State(state): State<AppState>,
) -> Json<Value> {
    state.chat_history.lock().await.clear();
    let vault = state.vault.lock().await;
    let _ = vault.clear_chat_history();
    Json(json!({"success": true}))
}

pub async fn start_new_session(
    State(state): State<AppState>,
) -> Json<Value> {
    let history = state.chat_history.lock().await;
    if history.is_empty() {
        return Json(json!({"success": true, "message": "History already empty"}));
    }

    // Format chat turns for summarizer prompt & serialize messages
    let mut turns = String::new();
    for msg in history.iter() {
        turns.push_str(&format!("{}: {}\n", msg.role, msg.content));
    }
    let messages_json = serde_json::to_string(&*history).ok();

    // Find the first *real* (non-command) user message for the title
    let first_real_msg = history.iter()
        .find(|m| m.role == "user" && !m.content.trim().starts_with('/'))
        .map(|m| m.content.as_str())
        .unwrap_or("");

    let title = if !first_real_msg.is_empty() {
        let words: Vec<&str> = first_real_msg.split_whitespace().take(5).collect();
        let candidate = words.join(" ");
        if candidate.len() > 40 {
            format!("{}...", &candidate[..40])
        } else {
            candidate
        }
    } else {
        "New Chat".to_string()
    };

    let summary = if !first_real_msg.is_empty() {
        if first_real_msg.len() > 80 {
            format!("{}...", &first_real_msg[..80])
        } else {
            first_real_msg.to_string()
        }
    } else {
        "Empty session".to_string()
    };

    // Only run async AI summarizer when there was a real conversation
    let has_real_exchange = !first_real_msg.is_empty();

    drop(history);

    // Save summary in database with heuristic first
    let is_pro = {
        let config = state.config.lock().await;
        config.vault.memory_mode == "pro"
    };

    let vault = state.vault.lock().await;

    // Check if we already have an active summary for this conversation — update it instead of creating a duplicate
    let existing_active_id = {
        let active_id = active_summary_id().lock().await;
        *active_id
    };

    let summary_id = if let Some(existing_id) = existing_active_id {
        // Update the existing session row with final title/summary/messages
        if let Some(ref json_str) = messages_json {
            let _ = vault.conn.execute(
                "UPDATE conversation_summaries SET title = ?1, summary = ?2, messages = ?3 WHERE id = ?4",
                rusqlite::params![title, summary, json_str, existing_id],
            );
        }
        existing_id
    } else {
        // No tracked session yet — create a new row
        match vault.save_summary(&title, &summary, messages_json.as_deref()) {
            Ok(_) => vault.conn.last_insert_rowid(),
            Err(_) => 0,
        }
    };
    
    // Clear in-memory chat history but keep it in the database for the old session
    state.chat_history.lock().await.clear();
    drop(vault);

    // Reset active summary ID so the next conversation gets a fresh session
    *active_summary_id().lock().await = None;

    // Spawn background task to update summary asynchronously if a model is loaded (Pro mode only to save resources)
    let active_model = {
        let config = state.config.lock().await;
        config.runtime.default_model.clone()
    };

    if !active_model.is_empty() && summary_id > 0 && is_pro && has_real_exchange {
        let ollama_host = {
            let config = state.config.lock().await;
            config.runtime.ollama_host.clone()
        };
        let vault_clone = Arc::clone(&state.vault);
        let title_fallback = title.clone();
        let summary_fallback = summary.clone();

        tokio::spawn(async move {
            let summarizer_prompt = format!(
                "Summarize this conversation in 1-2 sentences. Respond ONLY with a valid JSON object containing 'title' (a short 2-3 word topic name based on the chat contents) and 'summary' (the brief description). Example: {{\"title\": \"Rust Setup\", \"summary\": \"User asked about installing Rust on Arch Linux.\"}}\\n\\nConversation turns:\\n{}",
                turns
            );

            if let Ok(resp) = reqwest::Client::new()
                .post(format!("{}/api/generate", ollama_host))
                .json(&serde_json::json!({
                    "model": active_model,
                    "prompt": summarizer_prompt,
                    "stream": false,
                    "format": "json"
                }))
                .send()
                .await
            {
                if let Ok(body) = resp.json::<Value>().await {
                    if let Some(resp_text) = body["response"].as_str() {
                        if let Ok(parsed) = serde_json::from_str::<Value>(resp_text) {
                            let new_title = parsed["title"].as_str().unwrap_or(&title_fallback);
                            let new_summary = parsed["summary"].as_str().unwrap_or(&summary_fallback);

                            if let Ok(vault) = vault_clone.try_lock() {
                                let _ = vault.conn.execute(
                                    "UPDATE conversation_summaries SET title = ?1, summary = ?2 WHERE id = ?3",
                                    rusqlite::params![new_title, new_summary, summary_id],
                                );
                            } else {
                                // Fallback lock
                                let vault = vault_clone.lock().await;
                                let _ = vault.conn.execute(
                                    "UPDATE conversation_summaries SET title = ?1, summary = ?2 WHERE id = ?3",
                                    rusqlite::params![new_title, new_summary, summary_id],
                                );
                            }
                        }
                    }
                }
            }
        });
    }

    Json(json!({"success": true, "title": title, "summary": summary}))
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_exact_word_match() {
        assert!(is_exact_word_match("leo is a thief", "leo"));
        assert!(is_exact_word_match("leo is a thief", "thief"));
        assert!(is_exact_word_match("leo: thief", "thief"));
        assert!(!is_exact_word_match("leopold is a thief", "leo"));
        assert!(!is_exact_word_match("leo is a thief", "ie"));
    }

    #[test]
    fn test_score_memory_node() {
        let node = crate::vault::MemoryNode {
            id: 1,
            name: "harold likes god of war".to_string(),
            node_type: "manual".to_string(),
            description: "harold likes god of war ragnarock".to_string(),
        };

        // Matching keywords
        let score1 = score_memory_node(&node, &vec!["harold".to_string(), "war".to_string()]);
        assert!(score1 > 0);

        // Substring matching
        let score2 = score_memory_node(&node, &vec!["ragna".to_string()]);
        assert!(score2 > 0);
        
        // Zero score for non-matching
        let score3 = score_memory_node(&node, &vec!["sarah".to_string()]);
        assert_eq!(score3, 0);
    }
}

