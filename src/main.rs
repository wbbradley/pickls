// src/main.rs
use crate::prelude::*;

mod config;
mod diagnostic;
mod diagnostic_severity;
mod diagnostics_manager;
mod document_diagnostics;
mod document_storage;
mod document_version;
mod errno;
mod error;
mod job;
mod prelude;
mod tool;
mod utils;

struct PicklsServer {
    client: Client,
    jobs: Arc<Mutex<HashMap<JobId, Vec<Job>>>>,
    document_storage: Arc<Mutex<HashMap<Url, DocumentStorage>>>,
    config: Arc<Mutex<PicklsConfig>>,
    diagnostics_manager: DiagnosticsManager,
}

impl PicklsServer {
    pub fn new(client: Client, config: PicklsConfig) -> Self {
        Self {
            client: client.clone(),
            config: Arc::new(Mutex::new(config)),
            jobs: Arc::new(Mutex::new(Default::default())),
            document_storage: Arc::new(Mutex::new(Default::default())),
            diagnostics_manager: DiagnosticsManager::new(client),
        }
    }

    async fn get_site(&self) -> String {
        self.config.lock().await.site.clone()
    }

    async fn fetch_language_config(&self, language_id: &str) -> Option<PicklsLanguageConfig> {
        self.config.lock().await.languages.get(language_id).cloned()
    }

    async fn get_document(&self, uri: &Url) -> TowerLspResult<(String, Arc<String>)> {
        match self.document_storage.lock().await.get(&uri).cloned() {
            Some(DocumentStorage {
                language_id,
                file_contents,
            }) => Ok((language_id, file_contents)),
            None => TowerLspResult::Err(tower_lsp::jsonrpc::Error {
                code: tower_lsp::jsonrpc::ErrorCode::InvalidParams,
                message: format!("No document found for url '{uri}'").into(),
                data: None,
            }),
        }
    }

    async fn run_diagnostics(&self, job_spec: JobSpec) -> Result<()> {
        // Get a copy of the tool configuration for future use. Bail out if we
        // can't find it, this just means that the user doesn't want us to
        // run diagnostics for this language.
        let Some(language_config) = self.fetch_language_config(&job_spec.language_id).await else {
            log::info!(
                "no language config found for language_id={language_id}, skipping",
                language_id = job_spec.language_id
            );
            return Ok(());
        };

        // Lock the jobs structure while we manipulate it.
        let mut jobs = self.jobs.lock().await;

        let job_id = JobId::from(&job_spec);
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
                &language_config.root_markers,
                max_linter_count,
                file_content,
                job_spec.uri.clone(),
                job_spec.version,
            )
            .await?;
            debug_assert!(!jobs.contains_key(&job_id));
            new_jobs.push(Job { pid });
        }

        // Remember which jobs we started.
        assert!(jobs.insert(job_id, new_jobs).is_none());
        Ok(())
    }
}
type TowerLspResult<T> = tower_lsp::jsonrpc::Result<T>;

#[tower_lsp::async_trait]
impl LanguageServer for PicklsServer {
    async fn initialize(&self, params: InitializeParams) -> TowerLspResult<InitializeResult> {
        log::trace!(
            "[initialize called [pickls_pid={}, params={params:?}]",
            std::process::id()
        );
        if let Some(initialization_options) = params.initialization_options {
            let site = self.get_site().await;
            log::info!(
                "[PicklsServer in {site}] initialize updating configuration [{:?}]",
                initialization_options
            );
            update_configuration(&self.client, &self.config, initialization_options).await;
        }
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
                document_formatting_provider: Some(OneOf::Left(true)),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "pickls".to_string(),
                version: None,
            }),
        })
    }

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> TowerLspResult<Option<Vec<TextEdit>>> {
        log::info!("[formatting] called");

        let uri = params.text_document.uri;
        let (language_id, file_contents) = self.get_document(&uri).await?;
        let mut file_contents = file_contents.as_ref().clone();
        let language_config = match self.fetch_language_config(&language_id).await {
            Some(config) => config,
            None => {
                log::info!("No language config found for language ID {:?}", language_id);
                return Ok(None);
            }
        };

        // The big edit to return.
        let mut edit: Option<TextEdit> = None;
        log::info!(
            "Formatting file '{uri}' with {count} formatters",
            count = language_config.formatters.len()
        );
        for formatter_config in language_config.formatters {
            let program = formatter_config.program.clone();
            file_contents = match run_formatter(
                formatter_config,
                &language_config.root_markers,
                file_contents,
                uri.clone(),
            )
            .await
            {
                Ok(formatted_content) => {
                    log::info!("Formatter succeeded for url '{uri}' [formatter={program}]");
                    // Create a TextEdit that replaces the whole document
                    edit = Some(TextEdit {
                        range: Range {
                            start: Position::new(0, 0),
                            end: Position::new(u32::MAX, u32::MAX),
                        },
                        new_text: formatted_content.clone(),
                    });
                    formatted_content
                }
                Err(error) => {
                    log::error!("Formatter error: {:?}", error);
                    return TowerLspResult::Err(tower_lsp::jsonrpc::Error {
                        code: tower_lsp::jsonrpc::ErrorCode::InvalidParams,
                        message: format!("Formatter failed for url '{uri}' [formatter={program}]")
                            .into(),
                        data: None,
                    });
                }
            };
        }

        Ok(edit.map(|edit| vec![edit]))
    }

    async fn initialized(&self, _: InitializedParams) {
        let site = self.get_site().await;
        log::info!("[PicklsServer in {site}] initialized called");
        self.client
            .log_message(MessageType::INFO, "pickls Server initialized")
            .await;
    }

    async fn did_change_configuration(&self, dccp: DidChangeConfigurationParams) {
        if dccp.settings.is_null() {
            return;
        }
        if let serde_json::Value::Object(ref map) = dccp.settings {
            if map.is_empty() {
                return;
            }
        }
        update_configuration(&self.client, &self.config, dccp.settings).await;
    }

    async fn shutdown(&self) -> TowerLspResult<()> {
        let site = self.get_site().await;
        log::info!("[PicklsServer in {site}] shutdown called");
        Ok(())
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let site = self.get_site().await;
        self.document_storage
            .lock()
            .await
            .remove(&params.text_document.uri);
        log::info!("[PicklsServer in {site}] did_close called [params=...]");
    }
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let site = self.get_site().await;
        log::info!(
            "[PicklsServer in {site}] did_open called [language_id={language_id}, params=...]",
            language_id = params.text_document.language_id
        );
        let file_contents = Arc::new(params.text_document.text);
        self.document_storage.lock().await.insert(
            params.text_document.uri.clone(),
            DocumentStorage {
                language_id: params.text_document.language_id.clone(),
                file_contents: file_contents.clone(),
            },
        );
        if let Err(error) = self
            .run_diagnostics(JobSpec {
                uri: params.text_document.uri,
                version: DocumentVersion(params.text_document.version),
                language_id: params.text_document.language_id,
                text: file_contents,
            })
            .await
        {
            log::error!("did_open: {error:?}");
        }
    }
    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        let site = self.get_site().await;
        log::info!("[PicklsServer in {site}] did_change called [params=...]");
        assert!(params.content_changes.len() == 1);
        let file_contents = Arc::new(params.content_changes.remove(0).text);
        let uri = params.text_document.uri;

        let language_id = {
            let mut document_storage_map = self.document_storage.lock().await;
            let Some(document_storage) = document_storage_map.get_mut(&uri) else {
                self.client
                    .log_message(
                        MessageType::WARNING,
                        format!("no document found for uri {uri}"),
                    )
                    .await;
                return;
            };

            // Update the file contents.
            document_storage.file_contents = file_contents.clone();
            document_storage.language_id.clone()
        };

        if let Err(error) = self
            .run_diagnostics(JobSpec {
                uri,
                version: DocumentVersion(params.text_document.version),
                language_id,
                text: file_contents,
            })
            .await
        {
            log::warn!("did_change: {error:?}");
        }
    }
    async fn diagnostic(
        &self,
        _params: DocumentDiagnosticParams,
    ) -> TowerLspResult<DocumentDiagnosticReportResult> {
        log::trace!("[diagnostic] called");
        Ok(DocumentDiagnosticReportResult::Report(
            DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport::default(),
            }),
        ))
    }
}
async fn update_configuration(
    client: &Client,
    pickls_settings: &Arc<Mutex<PicklsConfig>>,
    settings: serde_json::Value,
) {
    match serde_json::from_value::<PicklsConfig>(settings) {
        Ok(settings) => {
            *pickls_settings.lock().await = settings.clone();
            client
                .log_message(
                    MessageType::INFO,
                    format!("configuration changed [config={settings:?}]!"),
                )
                .await;
        }
        Err(error) => {
            let message = format!("invalid pickls configuration [{error}]");
            log::warn!("{}", message);
            client.log_message(MessageType::WARNING, message).await;
        }
    }
}

fn get_state_dir() -> String {
    std::env::var("XDG_STATE_HOME").unwrap_or_else(|_| {
        [
            std::env::var("HOME")
                .expect("no HOME dir in environment")
                .as_str(),
            ".local",
            "state",
        ]
        .join("/")
    })
}

fn setup_logging(level: log::LevelFilter) -> Result<()> {
    let log_dir = format!("{state_dir}/pickls", state_dir = get_state_dir());
    std::fs::create_dir_all(&log_dir)?;
    simple_logging::log_to_file([log_dir.as_str(), "pickls.log"].join("/"), level)?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_logging(log::LevelFilter::Info)?;
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map_or(false, |arg| arg == "--version") {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let site = args.get(1).cloned();
    let parent_process_info = fetch_parent_process_info().await;
    log::info!(
        "pickls started; pid={pid}; parent_process_info={parent_process_info}",
        pid = nix::unistd::getpid()
    );
    let config_content: Option<String> = read_to_string("pickls.toml").ok();
    let mut config = config_content
        .as_ref()
        .map(|x| x.as_str())
        .map_or_else(Default::default, config::parse_config);
    // Initialize the configuration's site name.
    config.site = site.unwrap_or(config.site);
    let (service, socket) = LspService::build(|client| PicklsServer::new(client, config)).finish();
    Server::new(io::stdin(), io::stdout(), socket)
        .serve(service)
        .await;
    Ok(())
}
