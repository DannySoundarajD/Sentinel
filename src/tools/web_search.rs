// DuckDuckGo web search tool for Sentinel

use super::traits::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::json;
use regex::Regex;

pub struct WebSearchTool;

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web using DuckDuckGo to answer questions about recent events, facts, or search information."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query term"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let query = args["query"]
            .as_str()
            .or_else(|| args["arguments"]["query"].as_str())
            .or_else(|| args.as_str())
            .ok_or_else(|| anyhow::anyhow!("query parameter required"))?;

        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()?;

        let url = format!("https://www.mojeek.com/search?q={}", urlencoding::encode(query));
        
        let resp = match client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                return Ok(ToolResult {
                    output: format!("Failed to perform search request: {}", e),
                    success: false,
                });
            }
        };

        let html = match resp.text().await {
            Ok(t) => t,
            Err(e) => {
                return Ok(ToolResult {
                    output: format!("Failed to read search response body: {}", e),
                    success: false,
                });
            }
        };

        let mut results = Vec::new();
        let mut blocks = Vec::new();
        
        // Mojeek search result blocks are wrapped in <!--rs-->...<!--re--> tags.
        let re_block = Regex::new(r"(?s)<!--rs-->(.*?)<!--re-->").unwrap();
        for cap_block in re_block.captures_iter(&html) {
            blocks.push(cap_block.get(1).map(|m| m.as_str()).unwrap_or(""));
        }

        // Fallback: match <li> blocks with class r1, r2, ...
        if blocks.is_empty() {
            let re_li = Regex::new(r#"(?s)<li[^>]*class="r[0-9]+[^"]*"[^>]*>(.*?)</li>"#).unwrap();
            for cap in re_li.captures_iter(&html) {
                blocks.push(cap.get(1).map(|m| m.as_str()).unwrap_or(""));
            }
        }

        let re_any_url = Regex::new(r#"href="(?P<url>https?://[^"]+)""#).unwrap();
        let re_h2_text = Regex::new(r#"(?s)<h2>(?P<title_html>.*?)</h2>"#).unwrap();
        let re_snippet_p = Regex::new(r#"(?s)<p class="s"[^>]*>(?P<snippet>.*?)</p>"#).unwrap();

        for block in blocks {
            let url = re_any_url.captures(block)
                .and_then(|c| c.name("url"))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
                
            let mut title = String::new();
            if let Some(c) = re_h2_text.captures(block) {
                let html_inside = c.name("title_html").map(|m| m.as_str()).unwrap_or("");
                title = nanohtml2text::html2text(html_inside).trim().to_string();
            }
                
            let snippet_html = re_snippet_p.captures(block)
                .and_then(|c| c.name("snippet"))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            
            let snippet = nanohtml2text::html2text(&snippet_html).trim().to_string();
            
            if !title.is_empty() && !url.is_empty() {
                results.push(format!("Title: {}\nURL: {}\nSnippet: {}\n---", title, url, snippet));
            }

            if results.len() >= 5 {
                break;
            }
        }

        if results.is_empty() {
            Ok(ToolResult {
                output: "No results found. The search page layout might have changed or Mojeek might be rate-limiting. Try refining your query.".to_string(),
                success: false,
            })
        } else {
            Ok(ToolResult {
                output: results.join("\n\n"),
                success: true,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_web_search() {
        let tool = WebSearchTool;
        let result = tool.execute(serde_json::json!({"query": "rust language"})).await.unwrap();
        println!("Result success: {}, output len: {}", result.success, result.output.len());
        assert!(result.success, "Web search failed. Output: {}", result.output);
    }
}

