#![allow(dead_code)]
// src/main.rs
use crate::prelude::*;

mod config;
mod diagnostic;
mod diagnostic_severity;
mod diagnostics_manager;
mod document_diagnostics;
mod document_version;
mod errno;
mod error;
mod job;
mod prelude;
mod tool;
mod utils;

struct LintLsServer {
    client: Client,
    jobs: Arc<Mutex<HashMap<JobId, Vec<Job>>>>,
    config: Arc<Mutex<LintLsConfig>>,
    diagnostics_manager: DiagnosticsManager,
}

impl LintLsServer {
    pub fn new(client: Client, config: LintLsConfig) -> Self {
        Self {
            client: client.clone(),
            config: Arc::new(Mutex::new(config)),
            jobs: Arc::new(Mutex::new(Default::default())),
            diagnostics_manager: DiagnosticsManager::new(client),
        }
    }

    async fn fetch_language_config(&self, language_id: &str) -> Option<LintLsLanguageConfig> {
        self.config.lock().await.languages.get(language_id).cloned()
    }

    async fn run_diagnostics(&self, job_spec: JobSpec) -> Result<()> {
        let job_id = JobId::from(&job_spec);
        let Some(ref language_id) = job_spec.language_id else {
            log::info!("run_diagnostics called without a language_id");
            return Ok(());
        };

        // Get a copy of the tool configuration for future use.
        let language_config: LintLsLanguageConfig = self
            .fetch_language_config(language_id)
            .await
            .ok_or(Error::new(format!(
            "failed to get language_config from language_id [language_id={language_id}]"
        )))?;

        // Lock the jobs structure while we manipulate it.
        let mut jobs = self.jobs.lock().await;

        // Get rid of a prior running jobs.
        if let Some(jobs) = jobs.remove(&job_id) {
            for job in jobs {
                job.spawn_kill();
            }
        }

        let mut new_jobs: Vec<Job> = Default::default();
        let max_linter_count = language_config.linters.len();

        for linter_config in language_config.linters {
            let job_id: JobId = job_id.clone();
            let job_spec: JobSpec = job_spec.clone();
            let file_content = if linter_config.use_stdin {
                Some(job_spec.text.clone())
            } else {
                None
            };
            let pid: Pid = run_linter(
                self.diagnostics_manager.clone(),
                linter_config,
                max_linter_count,
                file_content,
                job_spec.uri.clone(),
                job_spec.version,
            )
            .await?;
            debug_assert!(!jobs.contains_key(&job_id));
            new_jobs.push(Job { job_spec, pid });
        }

        // Remember which jobs we started.
        assert!(jobs.insert(job_id, new_jobs).is_none());
        Ok(())
    }
}
type TowerResult<T> = tower_lsp::jsonrpc::Result<T>;

#[tower_lsp::async_trait]
impl LanguageServer for LintLsServer {
    async fn initialize(&self, _params: InitializeParams) -> TowerResult<InitializeResult> {
        log::info!("[initialize called [lintls_pid={}]", std::process::id());
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
                            work_done_progress: Some(false),
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
        log::info!("[did_change_configuration] called");
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
                let message = format!("invalid lintls configuration [{error}]");
                log::error!("{}", message);
                self.client.log_message(MessageType::ERROR, message).await;
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
                version: DocumentVersion(params.text_document.version),
                language_id: Some(params.text_document.language_id),
                text: Arc::new(params.text_document.text),
            })
            .await
        {
            log::error!("did_open: {error:?}");
        }
    }
    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        log::info!("[LintLsServer::did_change] called [params=...]");
        assert!(params.content_changes.len() == 1);
        let job_id = JobId(params.text_document.uri.clone());
        // NOTE: this is a little fragile, should find a better way of tracking language_ids.
        let language_id: Option<String> = {
            self.jobs
                .lock()
                .await
                .get(&job_id)
                .and_then(|jobs| jobs.first().map(|job| job.job_spec.language_id.clone()))
                .flatten()
        };
        if let Err(error) = self
            .run_diagnostics(JobSpec {
                uri: params.text_document.uri,
                version: DocumentVersion(params.text_document.version),
                language_id,
                text: Arc::new(params.content_changes.remove(0).text),
            })
            .await
        {
            log::warn!("did_change: {error:?}");
        }
    }
    async fn diagnostic(
        &self,
        _params: DocumentDiagnosticParams,
    ) -> TowerResult<DocumentDiagnosticReportResult> {
        log::info!("[diagnostic] called");
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
    let log_dir = std::env::var("XDG_STATE_HOME").unwrap_or_else(|_| {
        [
            std::env::var("HOME").expect("no HOME dir").as_str(),
            ".local",
            "state",
            "lintls",
        ]
        .join("/")
    });
    std::fs::create_dir_all(&log_dir)?;
    simple_logging::log_to_file(
        [log_dir.as_str(), "lintls.log"].join("/"),
        log::LevelFilter::Trace,
    )
    .unwrap();
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
