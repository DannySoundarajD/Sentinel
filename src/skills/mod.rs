// Sentinel Skills: Built-in skill collection

pub struct Skills;

impl Skills {
    pub fn builtin_skills() -> Vec<&'static str> {
        vec!["search", "calculate", "summarize"]
    }
}