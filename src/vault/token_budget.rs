/// Token budget management for context-aware prompting.
/// Helps fit context into small-context models on low-spec hardware.

#[derive(Debug, Clone)]
pub struct TokenBudget {
    pub model_context: u32,      // Total context window from model
    pub system_reserve: u32,     // Tokens reserved for system prompt (default 200)
    pub response_reserve: u32,   // Tokens reserved for model response (default 512)
}

impl TokenBudget {
    pub fn new(model_context: u32) -> Self {
        TokenBudget {
            model_context,
            system_reserve: 200,
            response_reserve: 512,
        }
    }

    pub fn with_reserves(model_context: u32, system_reserve: u32, response_reserve: u32) -> Self {
        TokenBudget {
            model_context,
            system_reserve,
            response_reserve,
        }
    }

    /// Calculate available tokens for context after reserves
    pub fn available(&self) -> u32 {
        self.model_context
            .saturating_sub(self.system_reserve)
            .saturating_sub(self.response_reserve)
    }
}

/// Rough token estimator: 1 token ≈ 4 characters
/// Good enough for budget calculations
pub fn estimate_tokens(text: &str) -> u32 {
    ((text.len() as u32 + 3) / 4).max(1)
}

/// Truncate text to fit within token budget, cutting from the FRONT
/// Keeps the most recent content, drops oldest
/// Tries to align to nearest newline to avoid cutting mid-sentence
pub fn truncate_to_budget(text: &str, max_tokens: u32) -> String {
    let max_chars = (max_tokens * 4) as usize;
    if text.len() <= max_chars {
        return text.to_string();
    }

    // Cut from front, keep tail (most recent)
    let cut_point = text.len() - max_chars;

    // Try to align to nearest newline to avoid cutting mid-sentence
    let aligned = text[cut_point..].find('\n')
        .map(|i| cut_point + i + 1)
        .unwrap_or(cut_point);

    format!("[...context trimmed for memory...]\n{}", &text[aligned..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_budget_available() {
        let budget = TokenBudget::new(4096);
        // 4096 - 200 (system) - 512 (response) = 3384
        assert_eq!(budget.available(), 3384);
    }

    #[test]
    fn test_estimate_tokens() {
        // 4 chars = 1 token
        assert_eq!(estimate_tokens("hello world"), 3); // "hello world" = 11 chars / 4 ≈ 3
    }

    #[test]
    fn test_truncate_to_budget_no_truncation() {
        let text = "hello world";
        let result = truncate_to_budget(text, 100);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_truncate_to_budget_with_truncation() {
        let text = "hello world\nthis is a longer text";
        let result = truncate_to_budget(text, 8); // 8 tokens = 32 chars
        assert!(result.contains("[...context trimmed"));
        assert!(result.contains("this is a longer text"));
    }
}
