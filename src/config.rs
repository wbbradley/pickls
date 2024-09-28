use crate::prelude::*;

#[derive(Clone, Debug, Deserialize, Default)]
pub struct LintLsConfig {
    pub languages: HashMap<String, LintLsLanguageConfig>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LintLsLanguageConfig {
    /// All the linters you'd like to run on this language. Each linter runs in a subprocess group.
    pub linters: Vec<LintLsLinterConfig>,
    /// All the formatters you'd like to run (in order) on this language. Note that you'll need to
    /// configure your editor to invoke its LSP client to cause formatting to occur. Successive
    /// formatters that set use_stdin will have chained pipes from stdout to stdin to eliminate extra
    /// copies.
    pub formatters: Vec<LintLsFormatterConfig>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LintLsLinterConfig {
    /// If `program` is not an absolute path, the `PATH` will be searched in an OS-defined way.
    pub program: String,
    /// Regex from which to pull diagnostics from stdout of `program`. The pattern is matched on
    /// every line of output. When there is a match, a diagnostic is produced.
    pub pattern: String,
    /// Regex group (1-indexed) that matches the filename of the diagnostic.
    pub filename_match: Option<usize>,
    /// Regex group (1-indexed) that matches the line number of the diagnostic.
    pub line_match: usize,
    /// Regex group (1-indexed) that matches the column number of the diagnostic. (Optional)
    pub col_match: Option<usize>,
    /// Regex group (1-indexed) that matches the line number of the diagnostic. Use -1 to indicate
    /// that the description is on the _previous_ line of input.
    pub description_match: Option<isize>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LintLsFormatterConfig {
    /// If `program` is not an absolute path, the `PATH` will be searched in an OS-defined way.
    pub program: String,
    /// Arguments to pass to `program`. Use "$abspath" wherever the absolute path to the filename should go.
    pub args: Vec<String>,
    /// Whether to use stdin to push the contents of the file to `program` or to rely on the usage
    /// of "$abspath" arg.
    pub use_stdin: bool,
}

pub fn parse_config(content: &str) -> LintLsConfig {
    toml::from_str(content).expect("Failed to parse TOML configuration")
}
