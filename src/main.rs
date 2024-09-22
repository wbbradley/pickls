// src/main.rs

use crate::config::{LintLspConfig, LintTool};
use nix::unistd::setsid;
use regex::Regex;
use std::fs::read_to_string;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io;
use tokio::process::Command;
use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::lsp_types::{Diagnostic as LspDiagnostic, DiagnosticSeverity, Position, Range};
use tower_lsp::{Client, LanguageServer, LspService, Server};
mod config;

pub struct LintLspDiagnostic {
    pub filename: String,
    pub line: u32,
    pub description: Option<String>,
}

impl LintLspDiagnostic {
    pub fn to_lsp_diagnostic(&self) -> LspDiagnostic {
        LspDiagnostic {
            range: Range {
                start: Position {
                    line: self.line.saturating_sub(1),
                    character: 0,
                },
                end: Position {
                    line: self.line.saturating_sub(1),
                    character: 0,
                },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("lintlsp".to_string()),
            message: self
                .description
                .clone()
                .unwrap_or_else(|| "error".to_string()),
            related_information: None,
            tags: None,
            data: None,
        }
    }
}

pub struct LintLspServer {
    client: Client,
    config: Arc<Mutex<LintLspConfig>>,
}

impl LintLspServer {
    pub fn new(client: Client, config: LintLspConfig) -> Self {
        Self {
            client,
            config: Arc::new(Mutex::new(config)),
        }
    }

    async fn run_diagnostics(&self, uri: Url, text: &str) {
        let mut sample = text.to_string();
        sample.truncate(100);
        log::info!(
            "[run_diagnostics] uri={uri}, language_id='{language_id}', text=\"{sample}...\""
        );
        if let Ok(path) = uri.to_file_path() {
            let file_extension = path.extension().and_then(|os_str| os_str.to_str());
            if let Some(extension) = file_extension {
                let tools = self.config.lock().await.tools.clone();
                for tool in tools
                    .iter()
                    .filter(|t| t.match_extensions.contains(&format!(".{}", extension)))
                {
                    let diagnostics = run_tool(tool, path.to_str().unwrap()).await;
                    // Convert to LSP Diagnostics
                    let lsp_diagnostics: Vec<Diagnostic> = diagnostics
                        .into_iter()
                        .map(|diag| diag.to_lsp_diagnostic())
                        .collect();

                    // Publish diagnostics to the client
                    self.client
                        .publish_diagnostics(uri.clone(), lsp_diagnostics, None)
                        .await;
                }
            }
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for LintLspServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        log::info!(
            "initialize called [params={:?}, lintlsp_pid={}]",
            params,
            std::process::id()
        );
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: None,
                        inter_file_dependencies: false,
                        workspace_diagnostics: false,
                        work_done_progress_options: WorkDoneProgressOptions {
                            work_done_progress: None,
                        },
                    },
                )),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "lintlsp".to_string(),
                version: None,
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "LintLSP Server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        log::info!("[LintLspServer::did_open] called [params={params:?}]");
        self.run_diagnostics(params.text_document.uri, &params.text_document.text)
            .await;
    }
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        log::info!("[LintLspServer::did_change] called [params={params:?}]");
        assert!(params.content_changes.len() == 1);
        self.run_diagnostics(params.text_document.uri, &(params.content_changes[0].text))
            .await;
    }

    // Implement other necessary methods like did_change or did_save if needed.
}

#[tokio::main]
async fn main() -> Result<()> {
    simple_logging::log_to_file("lintlsp.log", log::LevelFilter::Trace).unwrap();

    let config_content: Option<String> = read_to_string("config.toml").ok();
    let config =
        config_content.map_or_else(Default::default, |content| config::parse_config(&content));

    let stdin = io::stdin();
    let stdout = io::stdout();
    let (service, socket) = LspService::build(|client| LintLspServer::new(client, config)).finish();
    Server::new(stdin, stdout, socket).serve(service).await;
    Ok(())
}

async fn run_tool(tool: &LintTool, file_path: &str) -> Vec<LintLspDiagnostic> {
    let mut cmd = Command::new(&tool.path);
    cmd.arg(file_path).stdout(Stdio::piped());

    command_with_new_session(&mut cmd);

    let output = cmd
        .spawn()
        .unwrap_or_else(|e| {
            panic!(
                "Failed to execute tool [tool.path={}, error={e:?}]",
                tool.path
            )
        })
        .wait_with_output()
        .await
        .unwrap_or_else(|e| {
            panic!(
                "Failed to read tool output [tool.path={}, error={e:?}]",
                tool.path
            )
        });
    let stdout = String::from_utf8_lossy(&output.stdout);
    let re = Regex::new(&tool.pattern)
        .unwrap_or_else(|e| panic!("Invalid regex '{}' [error={e:?}", tool.pattern));
    stdout
        .lines()
        .filter_map(|line| {
            re.captures(line).map(|caps| LintLspDiagnostic {
                filename: caps.get(tool.filename_match).unwrap().as_str().to_string(),
                line: caps.get(tool.line_match).unwrap().as_str().parse().unwrap(),
                description: tool
                    .description_match
                    .map(|i| caps.get(i).unwrap().as_str().to_string()),
            })
        })
        .collect()
}

/// Ensure that this process creates its own session and process subgroup so that we can kill the
/// whole group.
fn command_with_new_session(cmd: &mut Command) -> &mut Command {
    unsafe {
        cmd.pre_exec(|| {
            setsid().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            Ok(())
        })
    }
}
