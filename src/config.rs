use crate::prelude::*;

#[derive(Clone, Debug, Deserialize, Default)]
pub struct PicklsConfig {
    pub languages: HashMap<String, PicklsLanguageConfig>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct PicklsLanguageConfig {
    /// All the linters you'd like to run on this language. Each linter runs in a subprocess group.
    #[serde(default)]
    pub linters: Vec<PicklsLinterConfig>,
    /// All the formatters you'd like to run (in order) on this language. Note that you'll need to
    /// configure your editor to invoke its LSP client to cause formatting to occur. Successive
    /// formatters that set use_stdin will have chained pipes from stdout to stdin to eliminate extra
    /// copies.
    #[serde(default)]
    pub formatters: Vec<PicklsFormatterConfig>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PicklsLinterConfig {
    /// If `program` is not an absolute path, the `PATH` will be searched in an OS-defined way.
    pub program: String,
    /// Arguments to pass to `program`. Use "$filename" wherever the absolute path to the real filename should go.
    /// Use "$tmpfilename" where Pickls should inject a temp file (if the linter only accepts file
    /// input).
    #[serde(default = "Vec::new")]
    pub args: Vec<String>,
    /// Whether to use stdin to push the contents of the file to `program` or to rely on the usage
    /// of "$filename" arg.
    pub use_stdin: bool,
    /// Regex from which to pull diagnostics from stdout of `program`. The pattern is matched on
    /// every line of output. When there is a match, a diagnostic is produced.
    pub pattern: String,
    /// Regex group (1-indexed) that matches the filename of the diagnostic.
    pub filename_match: Option<usize>,
    /// Regex group (1-indexed) that matches the line number of the diagnostic.
    pub line_match: usize,
    /// Regex group (1-indexed) that matches the starting column number of the diagnostic. (Optional)
    pub start_col_match: Option<usize>,
    /// Regex group (1-indexed) that matches the ending column number of the diagnostic. (Optional)
    pub end_col_match: Option<usize>,
    /// Regex group (1-indexed) that matches the severity of the alert. Unknown severities will
    /// resolve to warnings.
    pub severity_match: Option<usize>,
    /// Regex group (1-indexed) that matches the line number of the diagnostic. Use -1 to indicate
    /// that the description is on the _previous_ line of input.
    pub description_match: Option<isize>,
    /// Whether to scan stderr instead of stdout. Defaults to false. Setting to true will ignore
    /// stdout.
    #[serde(default = "default_false")]
    pub use_stderr: bool,
}

fn default_false() -> bool {
    false
}

#[derive(Clone, Debug, Deserialize)]
pub struct PicklsFormatterConfig {
    /// If `program` is not an absolute path, the `PATH` will be searched in an OS-defined way.
    pub program: String,
    /// Arguments to pass to `program`. Use "$abspath" wherever the absolute path to the filename should go.
    pub args: Vec<String>,
    /// Whether to use stdin to push the contents of the file to `program` or to rely on the usage
    /// of "$abspath" arg.
    pub use_stdin: bool,
}

pub fn parse_config(content: &str) -> PicklsConfig {
    toml::from_str(content).expect("Failed to parse TOML configuration")
}
