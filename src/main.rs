// src/main.rs
#![allow(clippy::too_many_arguments)]

use crate::prelude::*;

#[macro_use]
extern crate serde_json;

mod ai;
mod client;
mod diagnostic;
mod diagnostic_severity;
mod diagnostics_manager;
mod document_diagnostics;
mod document_storage;
mod document_version;
mod errno;
mod error;
mod job;
mod language_server;
mod prelude;
mod server;
mod tags;
mod tool;
mod utils;
mod workspace;

struct PicklsBackend {
    client: Client,
    rt: Runtime,
    client_info: Option<ClientInfo>,

    workspace: Workspace,
    jobs: HashMap<JobId, Vec<Job>>,
    document_storage: HashMap<Uri, DocumentStorage>,
    config: PicklsConfig,
    diagnostics_manager: DiagnosticsManager,
}

impl PicklsBackend {
    pub fn new(client: Client, rt: Runtime, config: PicklsConfig) -> Self {
        Self {
            rt,
            workspace: Workspace::new(),
            config,
            jobs: Default::default(),
            client_info: None,
            document_storage: Default::default(),
            diagnostics_manager: DiagnosticsManager::new(client.clone()),
            client,
        }
    }

    fn fetch_language_config(&self, language_id: &str) -> Option<PicklsLanguageConfig> {
        self.config.languages.get(language_id).cloned()
    }

    fn get_client_name(&self) -> String {
        if let Some(client_info) = self.client_info.as_ref() {
            format!(
                "{}{}{}",
                client_info.name,
                if client_info.version.is_some() {
                    "@"
                } else {
                    ""
                },
                client_info.version.as_deref().unwrap_or_default()
            )
        } else {
            String::from("Client?")
        }
    }

    fn get_workspace_name(&self) -> String {
        let name = self
            .workspace
            .folders()
            .filter_map(|folder| folder.file_name().map(|x| x.to_string_lossy()))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "{client_name}({workspace_name})",
            client_name = self.get_client_name(),
            workspace_name = if name.is_empty() {
                "<unknown>"
            } else {
                name.as_str()
            }
        )
    }

    fn get_document(&self, uri: &Uri) -> Result<DocumentStorage> {
        match self.document_storage.get(uri).cloned() {
            Some(ds) => Ok(ds),
            None => Err(Error::new(format!(
                "No document found for url '{uri}'",
                uri = uri.as_str()
            ))),
        }
    }

    fn run_diagnostics(&mut self, job_spec: JobSpec) -> Result<()> {
        // Get a copy of the tool configuration for future use. Bail out if we
        // can't find it, this just means that the user doesn't want us to
        // run diagnostics for this language.
        let Some(language_config) = self.fetch_language_config(&job_spec.language_id) else {
            log::trace!(
                "no language config found for language_id={language_id}, skipping",
                language_id = job_spec.language_id
            );
            return Ok(());
        };

        // Lock the jobs structure while we manipulate it.

        let job_id = JobId::from(&job_spec);
        // Get rid of a prior running jobs.
        if let Some(jobs) = self.jobs.remove(&job_id) {
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
                &mut self.diagnostics_manager,
                linter_config,
                &self.workspace,
                &language_config.root_markers,
                max_linter_count,
                file_content,
                job_spec.uri.clone(),
                job_spec.version,
            )?;
            debug_assert!(!self.jobs.contains_key(&job_id));
            new_jobs.push(Job { pid });
        }

        // Remember which jobs we started.
        assert!(self.jobs.insert(job_id, new_jobs).is_none());
        Ok(())
    }
    fn fetch_inline_assistance(
        &self,
        language_id: String,
        text: String,
    ) -> Result<InlineAssistResponse> {
        let (api_key_cmd, model) = {
            let openai_config = self
                .config
                .ai
                .openai
                .as_ref()
                .ok_or("No OpenAI configuration found")?;
            (
                openai_config.api_key_cmd.clone(),
                openai_config.model.clone(),
            )
        };

        let context = InlineAssistTemplateContext { language_id, text };
        let prompt = render_template(&self.config.ai.inline_assist.template, context)
            .context("Inline assist prompt is not properly configured")?;

        // Send this over yonder to the background thread.
        self.rt.block_on(async move {
            let api_key = get_command_output(&api_key_cmd)
                .await
                .context("getting api_key_cmd output")?;
            let mut openai_answer = fetch_completion(api_key, model, prompt).await?;
            log::info!("openai_answer: {:?}", openai_answer);
            Ok(InlineAssistResponse {
                code: std::mem::take(&mut openai_answer.choices[0].message.content),
            })
        })
    }
}

impl LanguageServer for PicklsBackend {
    fn initialize(&mut self, params: InitializeParams) -> Result<InitializeResult> {
        log::info!("[initialize called [pickls_pid={}]", std::process::id());
        self.client_info = params.client_info;
        if let Some(workspace_folders) = params.workspace_folders {
            for workspace_folder in workspace_folders {
                log::info!(
                    "adding folder: [name='{name}', uri='{uri}']",
                    name = workspace_folder.name,
                    uri = workspace_folder.uri.as_str()
                );
                self.workspace.add_folder(workspace_folder.uri);
            }
        };
        if let Some(initialization_options) = params.initialization_options {
            log::info!(
                "[PicklsBackend] initialize updating configuration [{initialization_options:?}]",
            );
            update_configuration(&self.client, &mut self.config, initialization_options)?;
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
                code_action_provider: Some(CodeActionProviderCapability::Options(
                    CodeActionOptions {
                        code_action_kinds: Some(vec![CodeActionKind::new("pickls.inline-assist")]),
                        work_done_progress_options: WorkDoneProgressOptions {
                            work_done_progress: Some(false),
                        },
                        resolve_provider: Some(false),
                    },
                )),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["pickls.inline-assist".to_string()],
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: Some(false),
                    },
                }),
                workspace_symbol_provider: if self.config.symbols.is_some() {
                    Some(OneOf::Right(WorkspaceSymbolOptions {
                        work_done_progress_options: WorkDoneProgressOptions {
                            // We don't support reporting progress for workspace symbols. Would probably need to be a heuristic.
                            work_done_progress: Some(false),
                        },
                        // We don't yet support symbol resolution.
                        resolve_provider: Some(false),
                    }))
                } else {
                    None
                },

                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "pickls".to_string(),
                version: None,
            }),
        })
    }
    fn code_action(&mut self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        log::info!("Got a textDocument/codeAction request: {params:?}");
        // Get the text of the document from the document storage.
        let uri = params.text_document.uri;
        let DocumentStorage {
            language_id,
            file_contents,
            version,
        } = self.get_document(&uri)?;
        // Write a function that takes the file contents and the range from within the params and
        // returns a slice of the file contents that corresponds to the range.
        let range: Range = params.range;
        let file_contents = file_contents.as_ref();
        let text = slice_range(file_contents, range);
        log::info!("Got a selection: {text}");
        if text.is_empty() {
            log::info!("No selection found, returning early");
            return Ok(None);
        }

        // Always create at least one progress message to denote the current update.
        let progress = make_progress_params("running inline-assist", uri.clone(), version, 0, 1);
        self.client.send_notification::<Progress, _>(progress)?;
        let completed_progress =
            make_progress_params("completed inline-assist", uri.clone(), version, 1, 1);

        let result = (|| {
            let response = self.fetch_inline_assistance(language_id, text)?;
            Ok(Some(vec![CodeActionOrCommand::CodeAction(CodeAction {
                title: "Inline Assist".to_string(),
                kind: Some(CodeActionKind::new("pickls.inline-assist")),
                edit: Some(WorkspaceEdit {
                    changes: Some(
                        [(
                            uri,
                            vec![TextEdit {
                                range,
                                new_text: response.code,
                            }],
                        )]
                        .into_iter()
                        .collect(),
                    ),
                    document_changes: None,
                    change_annotations: None,
                }),
                command: None,
                diagnostics: None,
                is_preferred: None,
                disabled: None,
                data: None,
            })]))
        })();
        self.client
            .send_notification::<Progress, _>(completed_progress)?;
        result
    }
    fn execute_command(&mut self, params: ExecuteCommandParams) -> Result<Option<Value>> {
        let _ = params;
        log::error!("Got a workspace/executeCommand request, but it is not implemented");
        Ok(None)
    }
    fn formatting(&mut self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        log::info!("[formatting] called");

        let uri = params.text_document.uri;
        let DocumentStorage {
            mut file_contents,
            language_id,
            ..
        } = self.get_document(&uri)?;
        let language_config = match self.fetch_language_config(&language_id) {
            Some(config) => config,
            None => {
                log::info!("No language config found for language ID {:?}", language_id);
                return Ok(None);
            }
        };

        // The big edit to return.
        log::info!(
            "Formatting file '{uri}' with {count} formatters",
            uri = uri.as_str(),
            count = language_config.formatters.len()
        );
        for formatter_config in language_config.formatters.iter() {
            let program = formatter_config.program.clone();
            file_contents = run_formatter(
                formatter_config,
                &self.workspace,
                &language_config.root_markers,
                file_contents,
                uri.clone(),
            )
            .inspect(|formatted_content| {
                log::info!(
                    "Formatter {program} succeeded for url '{uri}' \
                        [formatted_len={formatted_len}, formatter={program}]",
                    uri = uri.as_str(),
                    formatted_len = formatted_content.len(),
                );

                // Create a TextEdit that replaces the whole document
            })
            .context("formatter error")?;
        }
        Ok(Some(vec![TextEdit {
            range: Range {
                start: Position::new(0, 0),
                end: Position::new(u32::MAX, u32::MAX),
            },
            new_text: file_contents.clone(),
        }]))
    }

    fn initialized(&mut self, _: InitializedParams) -> Result<()> {
        log::info!(
            "[{site}] initialized called",
            site = self.get_workspace_name()
        );
        self.client
            .log_message(MessageType::INFO, "pickls Server initialized")
    }

    fn did_change_configuration(&mut self, dccp: DidChangeConfigurationParams) -> Result<()> {
        if dccp.settings.is_null() {
            return Ok(());
        }
        if let serde_json::Value::Object(ref map) = dccp.settings {
            if map.is_empty() {
                return Ok(());
            }
        }
        update_configuration(&self.client, &mut self.config, dccp.settings)
    }

    fn shutdown(&self) -> Result<()> {
        log::info!("[{site}] shutdown called", site = self.get_workspace_name());
        Ok(())
    }

    fn did_close(&mut self, params: DidCloseTextDocumentParams) -> Result<()> {
        self.document_storage.remove(&params.text_document.uri);
        log::info!(
            "[{site}] did_close called [params=...]",
            site = self.get_workspace_name()
        );
        Ok(())
    }
    fn did_open(&mut self, params: DidOpenTextDocumentParams) -> Result<()> {
        log::info!(
            "[{site}] did_open called [language_id={language_id}, params=...]",
            site = self.get_workspace_name(),
            language_id = params.text_document.language_id
        );
        let file_contents = params.text_document.text;
        self.document_storage.insert(
            params.text_document.uri.clone(),
            DocumentStorage {
                language_id: params.text_document.language_id.clone(),
                file_contents: file_contents.clone(),
                version: DocumentVersion(params.text_document.version),
            },
        );
        self.run_diagnostics(JobSpec {
            uri: params.text_document.uri,
            version: DocumentVersion(params.text_document.version),
            language_id: params.text_document.language_id,
            text: file_contents,
        })
        .context("did_open")
    }
    fn did_change(&mut self, mut params: DidChangeTextDocumentParams) -> Result<()> {
        log::trace!(
            "[{site}] did_change called [params=...]",
            site = self.get_workspace_name()
        );
        assert!(params.content_changes.len() == 1);
        let file_contents = params.content_changes.remove(0).text;
        let uri = params.text_document.uri;

        let language_id = {
            let Some(document_storage) = self.document_storage.get_mut(&uri) else {
                self.client
                    .log_message(
                        MessageType::WARNING,
                        format!("no document found for uri {uri}", uri = uri.as_str()),
                    )
                    .unwrap();
                return Ok(());
            };

            // Update the file contents.
            document_storage.file_contents = file_contents.clone();
            document_storage.language_id.clone()
        };

        self.run_diagnostics(JobSpec {
            uri,
            version: DocumentVersion(params.text_document.version),
            language_id,
            text: file_contents,
        })
        .context("did_change")
    }
    fn workspace_symbol(
        &mut self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        let ctags_timeout = {
            let config = &self.config;
            let Some(symbols_config) = &config.symbols else {
                log::info!("symbol: not enabled");
                return Ok(None);
            };
            Duration::from_millis(symbols_config.ctags_timeout_ms)
        };

        log::info!(
            "symbol: called with query: '{}' [cwd='{}']",
            params.query,
            std::env::current_dir().unwrap().display()
        );
        let folders = self.workspace.folders().cloned().collect::<Vec<_>>();
        let query = params.query;
        let symbols = find_symbols(
            &query,
            &folders,
            &vec![
                ".git",
                ".mypy_cache",
                "*.json",
                ".venv",
                "target",
                "node_modules",
            ]
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
            ctags_timeout,
        )
        .context("failed to find symbols")?;

        if symbols.is_empty() {
            log::info!("found no symbols");
            return Ok(None);
        }

        log::info!("found symbols count = {}", symbols.len());
        Ok(Some(symbols))
    }
}

fn update_configuration(
    client: &Client,
    pickls_settings: &mut PicklsConfig,
    settings: serde_json::Value,
) -> Result<()> {
    match serde_json::from_value::<PicklsConfig>(settings) {
        Ok(settings) => {
            client.log_message(
                MessageType::INFO,
                format!("configuration changed [config={settings:?}]!"),
            )?;
            *pickls_settings = settings;
        }
        Err(error) => {
            let message = format!("invalid pickls configuration [{error}]");
            log::warn!("{}", message);
            client.log_message(MessageType::WARNING, message)?;
        }
    }
    Ok(())
}

fn setup_logging(base_dirs: &xdg::BaseDirectories, level: log::LevelFilter) -> Result<()> {
    let log_file_path = base_dirs.place_state_file("pickls.log")?;
    simple_logging::log_to_file(log_file_path, level)?;
    Ok(())
}
fn read_config(base_dirs: &xdg::BaseDirectories) -> Option<PicklsConfig> {
    let config_filename = base_dirs.get_config_file(format!("{}.yaml", env!("CARGO_PKG_NAME")));
    log::info!("attempting to read configuration from {config_filename:?}");
    let config = parse_config(
        read_to_string(config_filename)
            .ok_or_log("failed to read configuration")?
            .as_str(),
    )
    .context("failed to parse YAML configuration")
    .ok_or_log("failed to parse configuration");
    log::info!(
        "configuration {}read.",
        if config.is_some() {
            "successfully "
        } else {
            "could not be "
        }
    );
    config
}

pub fn parse_config(content: &str) -> Result<PicklsConfig> {
    Ok(serde_yml::from_str(content)?)
}

fn main() -> Result<()> {
    if std::env::args().nth(1) == Some("version".to_string()) {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let base_dirs = xdg::BaseDirectories::with_prefix(env!("CARGO_PKG_NAME")).unwrap();
    setup_logging(&base_dirs, log::LevelFilter::Info)?;

    let parent_process_info = fetch_parent_process_info();
    log::info!(
        "pickls started; pid={pid}; parent_process_info={parent_process_info}",
        pid = nix::unistd::getpid()
    );
    let config = read_config(&base_dirs).unwrap();
    let rt = Runtime::new().context("creating tokio runtime")?;
    // Initialize the configuration's site name.
    run_server(|client| PicklsBackend::new(client, rt, config))
}
