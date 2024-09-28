#![allow(dead_code)]
// src/main.rs
use crate::prelude::*;

mod config;
mod diagnostic;
mod errno;
mod error;
mod job;
mod prelude;
mod tool;
mod utils;

struct LintLsServer {
    client: Client,
    config: Arc<Mutex<LintLsConfig>>,
    jobs: Arc<Mutex<HashMap<JobId, Vec<Job>>>>,
}

impl LintLsServer {
    pub fn new(client: Client, config: LintLsConfig) -> Self {
        Self {
            client,
            config: Arc::new(Mutex::new(config)),
            jobs: Arc::new(Mutex::new(Default::default())),
        }
    }

    async fn run_diagnostics(&self, job_spec: JobSpec) -> Result<()> {
        let job_id = JobId::from(&job_spec);
        let mut jobs = self.jobs.lock().await;

        // Get rid of a prior running jobs.
        if let Some(jobs) = jobs.remove(&job_id) {
            for job in jobs {
                job.spawn_kill();
            }
        }

        let tools = self.config.lock().await.tools.clone();
        let Some(extension) = get_extension_from_url(&job_spec.uri) else {
            return Err(Error::new(format!(
                "failed to get extension from uri [uri={uri}]",
                uri = job_spec.uri
            )));
        };

        let mut new_jobs: Vec<Job> = Default::default();
        for tool in tools.iter().filter(|t| {
            t.match_extensions
                .iter()
                .any(|match_extension| *match_extension == extension)
        }) {
            let job_id: JobId = job_id.clone();
            let job_spec: JobSpec = job_spec.clone();
            let pid: Pid =
                run_tool(&self.client, tool, job_spec.uri.clone(), job_spec.version).await?;
            debug_assert!(!jobs.contains_key(&job_id));
            new_jobs.push(Job { job_spec, pid });
        }
        jobs.insert(job_id, new_jobs);
        Ok(())
    }
}
type TowerResult<T> = tower_lsp::jsonrpc::Result<T>;

#[tower_lsp::async_trait]
impl LanguageServer for LintLsServer {
    async fn initialize(&self, _params: InitializeParams) -> TowerResult<InitializeResult> {
        log::info!("initialize called [lintls_pid={}]", std::process::id());
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
        log::info!("[initialized] called");
        self.client
            .log_message(MessageType::INFO, "lintls Server initialized")
            .await;
    }

    async fn did_change_configuration(&self, dccp: DidChangeConfigurationParams) {
        log::info!("[did_change_configuration] called {dccp:?}");
        match serde_json::from_value::<LintLsConfig>(dccp.settings) {
            Ok(config) => {
                *self.config.lock().await = config.clone();
                self.client
                    .log_message(
                        MessageType::INFO,
                        format!("configuration changed [config={config:?}]!"),
                    )
                    .await;
            }
            Err(error) => {
                self.client
                    .log_message(
                        MessageType::ERROR,
                        format!("invalid lintls configuration [{error}]"),
                    )
                    .await;
            }
        }
    }

    async fn shutdown(&self) -> TowerResult<()> {
        log::info!("[shutdown] called");
        Ok(())
    }

    async fn did_close(&self, _params: DidCloseTextDocumentParams) {
        log::info!("[LintLsServer::did_close] called [params=...]");
    }
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        log::info!("[LintLsServer::did_open] called [params=...]");

        if let Err(error) = self
            .run_diagnostics(JobSpec {
                uri: params.text_document.uri,
                version: params.text_document.version,
                language_id: Some(params.text_document.language_id),
                text: params.text_document.text,
            })
            .await
        {
            log::warn!("did_open: {error:?}");
        }
    }
    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        log::info!("[LintLsServer::did_change] called [params=...]");
        assert!(params.content_changes.len() == 1);
        if let Err(error) = self
            .run_diagnostics(JobSpec {
                uri: params.text_document.uri,
                version: params.text_document.version,
                language_id: None,
                text: params.content_changes.remove(0).text,
            })
            .await
        {
            log::warn!("did_change: {error:?}");
        }
    }
    async fn diagnostic(
        &self,
        params: DocumentDiagnosticParams,
    ) -> TowerResult<DocumentDiagnosticReportResult> {
        log::info!("[LintLsServer::diagnostic] called [params={params:?}]");
        Ok(DocumentDiagnosticReportResult::Report(
            DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport::default(),
            }),
        ))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    simple_logging::log_to_file("lintls.log", log::LevelFilter::Trace).unwrap();
    let parent_process_info = fetch_parent_process_info().await;
    log::info!(
        "lintls started; pid={pid}; parent_process_info={parent_process_info}",
        pid = nix::unistd::getpid()
    );
    let config_content: Option<String> = read_to_string("config.toml").ok();
    let config =
        config_content.map_or_else(Default::default, |content| config::parse_config(&content));

    let stdin = io::stdin();
    let stdout = io::stdout();
    let (service, socket) = LspService::build(|client| LintLsServer::new(client, config)).finish();
    Server::new(stdin, stdout, socket).serve(service).await;
    Ok(())
}
