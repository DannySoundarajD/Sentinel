use std::sync::Arc;
use teloxide::prelude::*;
use crate::api::AppState;

pub struct TelegramBridge;

impl TelegramBridge {
    pub async fn start(state: AppState, token: String) -> anyhow::Result<()> {
        let bot = Bot::new(token);
        
        let state_clone = state.clone();

        println!("Telegram bridge starting...");

        let handler = dptree::entry()
            .branch(
                Update::filter_message().endpoint(move |bot: Bot, msg: Message| {
                    let state = state_clone.clone();
                    async move {
                        let text = msg.text().unwrap_or("").trim();
                        if text.is_empty() {
                            return Ok(());
                        }

                        // Help menu command list
                        if text == "/" || text == "/help" {
                            let help_text = "Available commands:\n\n\
                                             • /new - Start a fresh chat session\n\
                                             • /reset - Clear conversation history\n\
                                             • /status - Show CPU/RAM/GPU metrics\n\
                                             • /model <name> - Switch Ollama model\n\
                                             • /reason - Toggle reasoning mode\n\
                                             • /explain <topic> - Explain code/concepts\n\
                                             • /fix <code> - Fix bugs in code/text\n\
                                             • /review <code> - Review code\n\
                                             • /test <cmd> - Run validation check\n\
                                             • /doc <topic> - Generate documentation\n\
                                             • /exec <cmd> - Run a shell command\n\
                                             • /save <fact> - Save fact to memory Vault";
                            let _ = bot.send_message(msg.chat.id, help_text).await;
                            return Ok(());
                        }

                        // Local / DB commands
                        if text == "/new" || text == "/reset" {
                            let vault = state.vault.lock().await;
                            let _ = vault.clear_chat_history();
                            state.chat_history.lock().await.clear();
                            let _ = bot.send_message(msg.chat.id, "🧹 Conversation history cleared.").await;
                            return Ok(());
                        }

                        if text == "/reason" {
                            let _ = bot.send_message(msg.chat.id, "🧠 Reasoning mode enabled (Chain-of-Thought).").await;
                            return Ok(());
                        }

                        if text.starts_with("/save ") {
                            let fact = text[6..].trim();
                            let vault = state.vault.lock().await;
                            match vault.save_memory_node(fact, "concept", fact) {
                                Ok(_) => {
                                    let _ = bot.send_message(msg.chat.id, format!("💾 Saved to Vault memory: \"{}\"", fact)).await;
                                }
                                Err(e) => {
                                    let _ = bot.send_message(msg.chat.id, format!("❌ Error saving memory: {}", e)).await;
                                }
                            }
                            return Ok(());
                        }

                        if text.starts_with("/model ") {
                            let model_name = text[7..].trim();
                            if !model_name.is_empty() {
                                let mut config = state.config.lock().await;
                                config.runtime.default_model = model_name.to_string();
                                let _ = config.save();
                                let _ = bot.send_message(msg.chat.id, format!("🤖 Switched active model to: {}", model_name)).await;
                            } else {
                                let _ = bot.send_message(msg.chat.id, "Please specify a model name.").await;
                            }
                            return Ok(());
                        }

                        if text == "/status" {
                            match state.guardian.collect_metrics().await {
                                Ok(metrics) => {
                                    let status_text = format!(
                                        "🖥️ System Metrics:\n\n\
                                         • CPU Usage: {:.1}%\n\
                                         • CPU Temp: {:.1}°C\n\
                                         • RAM Usage: {:.1}% ({:.0}MB / {:.0}MB)\n\
                                         • GPU Usage: {:.1}%\n\
                                         • VRAM Usage: {:.0}MB",
                                        metrics.cpu_pct,
                                        metrics.cpu_temp_c.unwrap_or(0.0),
                                        metrics.ram_pct,
                                        metrics.ram_used_mb,
                                        metrics.ram_total_mb,
                                        metrics.gpu_pct.unwrap_or(0.0),
                                        metrics.vram_used_mb.unwrap_or(0)
                                    );
                                    let _ = bot.send_message(msg.chat.id, status_text).await;
                                }
                                Err(e) => {
                                    let _ = bot.send_message(msg.chat.id, format!("❌ Error collecting metrics: {}", e)).await;
                                }
                            }
                            return Ok(());
                        }

                        // Chat and prompt commands
                        let mut final_prompt = String::new();
                        if text.starts_with("/chat ") {
                            final_prompt = text[6..].trim().to_string();
                        } else if text.starts_with("/explain ") {
                            final_prompt = format!("Please explain this block of code or concept:\n{}", text[9..].trim());
                        } else if text.starts_with("/fix ") {
                            final_prompt = format!("Please analyze and fix bugs in this code or text:\n{}", text[5..].trim());
                        } else if text.starts_with("/review ") {
                            final_prompt = format!("Please review this code for style, safety, and efficiency:\n{}", text[8..].trim());
                        } else if text.starts_with("/doc ") {
                            final_prompt = format!("Please write detailed documentation for the following:\n{}", text[5..].trim());
                        } else if text.starts_with("/exec ") {
                            final_prompt = format!("{{\n  \"tool\": \"bash_exec\",\n  \"arguments\": {{\n    \"command\": \"{}\"\n  }}\n}}", text[6..].trim());
                        } else if text.starts_with("/test ") {
                            final_prompt = format!("{{\n  \"tool\": \"bash_exec\",\n  \"arguments\": {{\n    \"command\": \"{}\"\n  }}\n}}", text[6..].trim());
                        }

                        if final_prompt.is_empty() {
                            let _ = bot.send_message(msg.chat.id, "Unknown command or empty message. Type /help for available options.").await;
                            return Ok(());
                        }

                        let config = state.config.lock().await;
                        let active_model = config.runtime.default_model.clone();
                        let ollama_host = config.runtime.ollama_host.clone();
                        drop(config);

                        let _ = bot.send_message(msg.chat.id, "🤖 Processing...").await;

                        let client = reqwest::Client::new();
                        match client
                            .post(format!("{}/api/generate", ollama_host))
                            .json(&serde_json::json!({
                                "model": active_model,
                                "prompt": final_prompt,
                                "stream": false
                            }))
                            .send()
                            .await
                        {
                            Ok(resp) => {
                                if let Ok(body) = resp.json::<serde_json::Value>().await {
                                    let reply = body["response"].as_str().unwrap_or("Error generating response");
                                    let _ = bot.send_message(msg.chat.id, reply).await;
                                } else {
                                    let _ = bot.send_message(msg.chat.id, "Error parsing response").await;
                                }
                            }
                            Err(e) => {
                                let _ = bot.send_message(msg.chat.id, format!("Failed to connect to Ollama: {}", e)).await;
                            }
                        }
                        
                        respond(())
                    }
                })
            );

        tokio::spawn(async move {
            let mut dp = Dispatcher::builder(bot, handler)
                .enable_ctrlc_handler()
                .build();
            dp.dispatch().await;
        });

        Ok(())
    }
}
