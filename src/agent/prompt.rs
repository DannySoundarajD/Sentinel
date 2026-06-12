// Prompt Assembly

pub struct Prompt;

impl Prompt {
    pub fn build(user_input: &str, context: &str) -> String {
        format!("{}\n\nContext:\n{}", user_input, context)
    }
}
