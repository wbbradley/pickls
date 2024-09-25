#![allow(dead_code)]
// src/main.rs

use crate::config::{LintLsConfig, LintTool};
use crate::error::Result;
use nix::unistd::{getpid, setpgid};
use regex::Regex;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::sync::Arc;
use tokio::io;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::sync::Mutex;
use tower_lsp::lsp_types::*;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use tower_lsp::{Client, LanguageServer, LspService, Server};
mod config;
mod error;

pub struct LintLsDiagnostic {
    pub filename: String,
    pub line: u32,
    pub description: Option<String>,
}

impl From<LintLsDiagnostic> for Diagnostic {
    fn from(diagnostic: LintLsDiagnostic) -> Self {
        Self {
            range: Range {
                start: Position {
                    line: diagnostic.line.saturating_sub(1),
                    character: 0,
                },
                end: Position {
                    line: diagnostic.line.saturating_sub(1),
                    character: 0,
                },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("lintls".to_string()),
            message: diagnostic
                .description
                .clone()
                .unwrap_or_else(|| "error".to_string()),
            related_information: None,
            tags: None,
            data: None,
        }
    }
}

#[derive(Hash, Ord, PartialOrd, Eq, PartialEq)]
struct JobId {
    uri: Url,
}

impl From<&JobSpec> for JobId {
    fn from(js: &JobSpec) -> JobId {
        JobId {
            uri: js.uri.clone(),
        }
    }
}

enum JobState {
    Running { pid: u32 },
    Done,
}
struct JobSpec {
    uri: Url,
    version: i32,
    language_id: Option<String>,
    text: String,
}

struct Job {
    job_spec: JobSpec,
    job_state: JobState,
}

struct LintLsServer {
    client: Client,
    config: Arc<Mutex<LintLsConfig>>,
    jobs: Arc<Mutex<HashMap<JobId, Job>>>,
}

impl LintLsServer {
    pub fn new(client: Client, config: LintLsConfig) -> Self {
        Self {
            client,
            config: Arc::new(Mutex::new(config)),
            jobs: Arc::new(Mutex::new(Default::default())),
        }
    }

    async fn run_diagnostics(&self, job_spec: JobSpec) {
        let mut jobs = self.jobs.lock().await;
        let job_id = JobId::from(&job_spec);
        // HashMap<JobId, Job>
        let tools = self.config.lock().await.tools.clone();
        for tool in tools
            .iter()
            .filter(|t| t.match_extensions.contains(&format!(".{}", extension)))
        {}

        jobs.entry(job_id).or_insert_with(move || {
            let pid = 0;
            Job {
                job_spec,
                job_state: JobState::Running { pid },
            }
        });
        /*
                let mut sample = job_spec.text.clone();
                sample.truncate(100);
                log::info!(
                    "[run_diagnostics] uri={job_spec.uri}, language_id='{language_id}', text=\"{sample}...\""
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
        */
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for LintLsServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        log::info!(
            "initialize called [params={:?}, lintls_pid={}]",
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
                name: "lintls".to_string(),
                version: None,
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "lintls Server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        log::info!("[LintLsServer::did_open] called [params={params:?}]");

        self.run_diagnostics(JobSpec {
            uri: params.text_document.uri,
            version: params.text_document.version,
            language_id: Some(params.text_document.language_id),
            text: params.text_document.text,
        })
        .await;
    }
    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        log::info!("[LintLsServer::did_change] called [params={params:?}]");
        assert!(params.content_changes.len() == 1);
        self.run_diagnostics(JobSpec {
            uri: params.text_document.uri,
            version: params.text_document.version,
            language_id: None,
            text: params.content_changes.remove(0).text,
        })
        .await;
    }

    // Implement other necessary methods like did_change or did_save if needed.
}

#[tokio::main]
async fn main() -> Result<()> {
    simple_logging::log_to_file("lintls.log", log::LevelFilter::Trace).unwrap();

    let config_content: Option<String> = read_to_string("config.toml").ok();
    let config =
        config_content.map_or_else(Default::default, |content| config::parse_config(&content));

    let stdin = io::stdin();
    let stdout = io::stdout();
    let (service, socket) = LspService::build(|client| LintLsServer::new(client, config)).finish();
    Server::new(stdin, stdout, socket).serve(service).await;
    Ok(())
}

async fn run_tool(client: &Client, tool: &LintTool, file_path: &str) -> Result<u32> {
    // Result<Vec<LintLsDiagnostic>> {
    let mut cmd = Command::new(&tool.path);

    // Ensure that the child process creates its own process group so that we can kill the whole group.
    unsafe {
        cmd.pre_exec(|| {
            setpgid(getpid(), getpid()).expect("Failed to set new process group");
            Ok(())
        });
    }

    let mut child = cmd
        .arg("--with-stdin-path")
        .arg(file_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;

    let stdin: tokio::process::ChildStdin = child.stdin.take().expect("Failed to open stdin");
    let stdout: tokio::process::ChildStdout = child.stdout.take().expect("Failed to open stdout");
    let tool = tool.clone();
    let client = client.clone();
    tokio::spawn(async move { ingest_errors(client, tool, stdin, stdout).await.unwrap() });
    Ok(child.id().unwrap())
}

async fn ingest_errors(
    client: Client,
    tool: LintTool,
    mut stdin: tokio::process::ChildStdin,
    stdout: tokio::process::ChildStdout,
) -> Result<()> {
    let re = Regex::new(&tool.pattern)
        .unwrap_or_else(|e| panic!("Invalid regex [pattern={pattern}]", pattern = tool.pattern));
    stdin.write_all(b"").await;
    let mut reader = BufReader::new(stdout).lines();
    while let Some(line) = reader.next_line().await? {
        if let Some(diagnostic) = re.captures(line).map(|caps| LintLsDiagnostic {
            filename: caps.get(tool.filename_match).unwrap().as_str().to_string(),
            line: caps.get(tool.line_match).unwrap().as_str().parse().unwrap(),
            description: tool
                .description_match
                .map(|i| caps.get(i).unwrap().as_str().to_string()),
        }) {
            let lsp_diagnostic: Diagnostic = diagnostic.into();

            // Publish diagnostics to the client
            client
                .publish_diagnostics(uri.clone(), lsp_diagnostics, None)
                .await;
        }
    }
    Ok(())
}
