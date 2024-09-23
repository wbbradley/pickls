// src/config.rs
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct LintLsConfig {
    pub tools: Vec<LintTool>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LintTool {
    pub match_extensions: Vec<String>,
    pub path: String,
    pub pattern: String,
    pub filename_match: usize,
    pub line_match: usize,
    pub description_match: Option<usize>,
}

pub fn parse_config(content: &str) -> LintLsConfig {
    toml::from_str(content).expect("Failed to parse TOML configuration")
}
