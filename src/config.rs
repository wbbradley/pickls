// src/config.rs
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct LintLsConfig {
    pub tools: Vec<LintTool>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LintTool {
    pub match_extensions: Vec<String>,
    /// If `program` is not an absolute path, the `PATH` will be searched in an OS-defined way.
    pub program: String,
    /// Regex from which to pull diagnostics.
    pub pattern: String,
    /// Regex group (1-indexed) that matches the filename of the diagnostic.
    pub filename_match: Option<usize>,
    /// Regex group (1-indexed) that matches the line number of the diagnostic.
    pub line_match: usize,
    pub description_match: Option<usize>,
}

pub fn parse_config(content: &str) -> LintLsConfig {
    toml::from_str(content).expect("Failed to parse TOML configuration")
}
