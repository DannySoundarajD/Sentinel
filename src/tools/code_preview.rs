// Code Preview & Execution Tool
use std::process::Command;
use serde_json::json;

pub struct CodePreviewTool;

impl CodePreviewTool {
    /// Execute code and return output (supports: rust, python, javascript, bash, html)
    pub fn execute(language: &str, code: &str) -> String {
        match language {
            "python" | "py" => Self::execute_python(code),
            "javascript" | "js" => Self::execute_javascript(code),
            "rust" => Self::execute_rust(code),
            "bash" | "sh" => Self::execute_bash(code),
            "html" => Self::preview_html(code),
            _ => format!("Unsupported language: {}", language),
        }
    }

    fn execute_python(code: &str) -> String {
        match Command::new("python3")
            .arg("-c")
            .arg(code)
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                if !stderr.is_empty() {
                    format!("Error:\n{}\n\nOutput:\n{}", stderr, stdout)
                } else {
                    stdout
                }
            }
            Err(e) => format!("Failed to execute: {}", e),
        }
    }

    fn execute_javascript(code: &str) -> String {
        match Command::new("node")
            .arg("-e")
            .arg(code)
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                if !stderr.is_empty() {
                    format!("Error:\n{}\n\nOutput:\n{}", stderr, stdout)
                } else {
                    stdout
                }
            }
            Err(e) => format!("Failed to execute: {}", e),
        }
    }

    fn execute_rust(code: &str) -> String {
        let wrapped = format!(
            "fn main() {{\n    {}\n}}",
            code.replace('\n', "\n    ")
        );

        match Command::new("rustc")
            .arg("--crate-type")
            .arg("bin")
            .arg("-")
            .arg("-o")
            .arg("/tmp/rust_exec")
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            Ok(mut child) => {
                use std::io::Write;
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(wrapped.as_bytes());
                }
                match child.wait_with_output() {
                    Ok(output) => {
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        if !stderr.is_empty() {
                            format!("Compilation Error:\n{}", stderr)
                        } else {
                            match Command::new("/tmp/rust_exec").output() {
                                Ok(out) => String::from_utf8_lossy(&out.stdout).to_string(),
                                Err(e) => format!("Execution failed: {}", e),
                            }
                        }
                    }
                    Err(e) => format!("Compilation failed: {}", e),
                }
            }
            Err(e) => format!("Failed to compile: {}", e),
        }
    }

    fn execute_bash(code: &str) -> String {
        match Command::new("bash")
            .arg("-c")
            .arg(code)
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                if !stderr.is_empty() {
                    format!("Error:\n{}\n\nOutput:\n{}", stderr, stdout)
                } else {
                    stdout
                }
            }
            Err(e) => format!("Failed to execute: {}", e),
        }
    }

    fn preview_html(code: &str) -> String {
        // Save to temp file and return preview URL
        use std::fs;
        let temp_path = "/tmp/sentinel_preview.html";
        match fs::write(temp_path, code) {
            Ok(_) => format!(
                "HTML preview saved to: {}\nOpen in browser: file://{}\n\nContent:\n{}",
                temp_path, temp_path, code
            ),
            Err(e) => format!("Failed to save preview: {}", e),
        }
    }

    /// Analyze code and return metrics
    pub fn analyze(language: &str, code: &str) -> serde_json::Value {
        let lines = code.lines().count();
        let chars = code.len();
        let functions = Self::count_functions(language, code);
        let classes = Self::count_classes(language, code);

        json!({
            "language": language,
            "lines": lines,
            "characters": chars,
            "functions": functions,
            "classes": classes,
            "complexity": Self::estimate_complexity(language, code),
            "estimate_tokens": lines * 4, // Rough estimate
        })
    }

    fn count_functions(language: &str, code: &str) -> usize {
        match language {
            "python" | "py" => code.matches("def ").count(),
            "javascript" | "js" => {
                code.matches("function ").count() + code.matches("=>").count()
            }
            "rust" => code.matches("fn ").count(),
            _ => 0,
        }
    }

    fn count_classes(language: &str, code: &str) -> usize {
        match language {
            "python" | "py" => code.matches("class ").count(),
            "javascript" | "js" => code.matches("class ").count(),
            "rust" => code.matches("impl ").count(),
            _ => 0,
        }
    }

    fn estimate_complexity(_language: &str, code: &str) -> String {
        let condition_count = code.matches("if ").count()
            + code.matches("else").count()
            + code.matches("match").count()
            + code.matches("for ").count()
            + code.matches("while ").count();

        match condition_count {
            0..=3 => "Low".to_string(),
            4..=8 => "Medium".to_string(),
            9..=15 => "High".to_string(),
            _ => "Very High".to_string(),
        }
    }

    /// Format and beautify code
    pub fn format(language: &str, code: &str) -> String {
        match language {
            "python" | "py" => {
                // Basic Python formatting
                code.lines()
                    .map(|line| {
                        if line.trim().is_empty() {
                            String::new()
                        } else {
                            format!("{}{}", " ".repeat(Self::get_indent(line)), line.trim())
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            "javascript" | "js" => {
                // Basic JS formatting (add semicolons)
                code.lines()
                    .map(|line| {
                        let trimmed = line.trim();
                        if !trimmed.is_empty()
                            && !trimmed.ends_with(';')
                            && !trimmed.ends_with('{')
                            && !trimmed.ends_with('}')
                            && !trimmed.ends_with(',')
                        {
                            format!("{};", trimmed)
                        } else {
                            trimmed.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            _ => code.to_string(),
        }
    }

    fn get_indent(line: &str) -> usize {
        line.len() - line.trim_start().len()
    }
}
