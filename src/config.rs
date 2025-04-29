use serde::Deserialize;
use std::collections::HashMap;

const DEFAULT_CTAGS_TIMEOUT_MS: u64 = 500;

#[derive(Clone, Debug, Deserialize, Default)]
pub struct PicklsConfig {
    #[serde(default)]
    pub languages: HashMap<String, PicklsLanguageConfig>,
    pub symbols: Option<PicklsSymbolsConfig>,
    #[serde(default)]
    pub ai: PicklsAIConfig,
}

fn default_ctags_timeout_ms() -> u64 {
    DEFAULT_CTAGS_TIMEOUT_MS
}

#[derive(Eq, PartialEq, Clone, Debug, Deserialize)]
pub struct PicklsSymbolsConfig {
    pub source: PicklsSymbolsSource,

    /// How long to wait for ctags to complete before timing out. Defaults to 500ms.
    #[serde(default = "default_ctags_timeout_ms")]
    pub ctags_timeout_ms: u64,
}

#[derive(Eq, PartialEq, Clone, Debug, Deserialize)]
pub enum PicklsSymbolsSource {
    #[serde(rename = "universal-ctags")]
    UniversalCtags,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct PicklsLanguageConfig {
    /// A list of pathnames that indicate the root directory in relation to a file
    /// being processed. pickls will use the first directory containing one of
    /// these files as the root directory. The associated linter or formatter
    /// will be run with its working directory set to this directory. (ie: pyproject.toml,
    /// setup.py, Cargo.toml, go.mod, Makefile, etc...)
    #[serde(default)]
    pub root_markers: Vec<String>,

    /// All the linters you'd like to run on this language. Each linter runs in
    /// a subprocess group.
    #[serde(default)]
    pub linters: Vec<PicklsLinterConfig>,

    /// All the formatters you'd like to run (in order) on this language. Note
    /// that you'll need to configure your editor to invoke its LSP client to
    /// cause formatting to occur. Successive formatters that set use_stdin will
    /// have chained pipes from stdout to stdin to eliminate extra copies.
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

fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize)]
pub struct PicklsFormatterConfig {
    /// If `program` is not an absolute path, the `PATH` will be searched in an OS-defined way.
    pub program: String,
    /// Arguments to pass to `program`. Use "$abspath" wherever the absolute path to the filename should go.
    pub args: Vec<String>,
    /// Whether to use stdin to push the contents of the file to `program` or to rely on the usage
    /// of "$filename" arg. Defaults to true.
    #[serde(default = "default_true")]
    pub use_stdin: bool,
    /// If `stderr_indicates_error` is true, then if the formatter writes anything to stderr, the
    /// format run will be considered a failure and aborted. Defaults to false.
    #[serde(default = "default_false")]
    pub stderr_indicates_error: bool,
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct PicklsAIProviderModelRef {
    pub provider: Provider,
    pub model: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PicklsAIConfig {
    #[serde(default = "default_inline_assist_system_prompt")]
    pub system_prompt: String,
    pub inline_assistants: Vec<PicklsAIProviderModelRef>,
    #[serde(default = "default_inline_assist_prompt_template")]
    pub inline_assistant_prompt_template: String,
    #[serde(default = "default_false")]
    pub inline_assistant_include_workspace_files: bool,
    #[serde(default)]
    pub openai: OpenAIConfig,
    #[serde(default)]
    pub ollama: OllamaConfig,
}

impl Default for PicklsAIConfig {
    fn default() -> Self {
        PicklsAIConfig {
            system_prompt: default_inline_assist_system_prompt(),
            inline_assistants: Vec::new(),
            inline_assistant_prompt_template: default_inline_assist_prompt_template(),
            inline_assistant_include_workspace_files: true,
            openai: OpenAIConfig::default(),
            ollama: OllamaConfig::default(),
        }
    }
}

/// Ollama is a AI model driver that can be run locally.
/// See https://ollama.com/ for more information on getting it set up locally.
///
/// API docs are [here](https://github.com/ollama/ollama/blob/main/docs/api.md#generate-a-completion).
/// curl http://localhost:11434/api/generate -d '{
///   "model": "llama3.2",
///   "prompt": "Why is the sky blue?",
///   "system": "You are a good robot."
/// }'
#[derive(Clone, Debug, Deserialize)]
pub struct OllamaConfig {
    /// Defaults to http://localhost:11434/api/generate.
    #[serde(default = "default_ollama_api_address")]
    pub api_address: String,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        OllamaConfig {
            api_address: default_ollama_api_address(),
        }
    }
}

fn default_ollama_api_address() -> String {
    "http://localhost:11434/api/generate".to_string()
}

#[derive(Clone, Debug, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    #[default]
    OpenAI,
    Ollama,
}

#[derive(Clone, Debug, Deserialize)]
pub struct OpenAIConfig {
    /// The command to run to print the OpenAPI key. (If None, will look at $OPENAI_API_KEY)
    #[serde(default = "default_openai_api_key_cmd")]
    pub api_key_cmd: Vec<String>,
}

impl Default for OpenAIConfig {
    fn default() -> Self {
        OpenAIConfig {
            api_key_cmd: default_openai_api_key_cmd(),
        }
    }
}

fn default_openai_api_key_cmd() -> Vec<String> {
    ["sh", "-c", "echo $OPENAI_API_KEY"]
        .into_iter()
        .map(|s| s.to_string())
        .collect()
}

fn default_inline_assist_prompt_template() -> String {
    "\
{{#if include_workspace_files }}
{{#each files}}
------
File: {{@key}}

{{this}}
{{/each}}
------
{{/if}}
Notes: We're working within the {{language_id}} language. If I show you code below, then please
rewrite it to make improvements as you see fit. If I show you a question or directive,
write code to satisfy the question or directive. Never use markdown to format your response.
Never print ```. Always include type annotations where possible.

------
{{text}}\n"
        .to_string()
}

const INLINE_ASSIST_SYSTEM_PROMPT: &str = "You are a helpful inline code assistant. Reply concisely. Never include markdown (like ```) in your response.";

fn default_inline_assist_system_prompt() -> String {
    INLINE_ASSIST_SYSTEM_PROMPT.to_string()
}
