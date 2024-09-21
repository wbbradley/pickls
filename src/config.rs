// src/config.rs
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct LintLspConfig {
    pub tools: Vec<LintTool>,
}

#[derive(Debug, Deserialize)]
pub struct LintTool {
    pub match_extensions: Vec<String>,
    pub name: String,
    pub path: String,
    pub pattern: String,
    pub filename_match: usize,
    pub line_match: usize,
    pub col_match: Option<usize>,
    pub description_match: Option<usize>,
}

pub fn parse_config(content: &str) -> LintLspConfig {
    toml::from_str(content).expect("Failed to parse TOML configuration")
}
