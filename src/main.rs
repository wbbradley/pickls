// src/main.rs
#![allow(clippy::too_many_arguments)]

use crate::prelude::*;
#[macro_use]
extern crate serde_json;

mod client;
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
mod language_server;
mod prelude;
mod server;
mod tags;
mod tool;
mod utils;
mod workspace;

struct PicklsBackend<'a> {
    client: &'a Client,
    client_info: Option<ClientInfo>,

    workspace: Workspace,
    jobs: HashMap<JobId, Vec<Job>>,
    document_storage: HashMap<Uri, DocumentStorage>,
    config: PicklsConfig,
    diagnostics_manager: DiagnosticsManager<'a>,
}

impl<'a> PicklsBackend<'a> {
    pub fn new(client: &'a Client, config: PicklsConfig) -> Self {
        Self {
            client,
            workspace: Workspace::new(),
            config,
            jobs: Default::default(),
            client_info: None,
            document_storage: Default::default(),
            diagnostics_manager: DiagnosticsManager::new(&client),
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
}

impl<'a> LanguageServer for PicklsBackend<'a> {
    fn initialize(&mut self, params: InitializeParams) -> Result<InitializeResult> {
        log::info!("[initialize called [pickls_pid={}]", std::process::id(),);
        let client_info = &mut self.client_info;
        *client_info = params.client_info;
        if let (Some(workspace_folders), ref mut workspace) =
            (params.workspace_folders, self.workspace)
        {
            for workspace_folder in workspace_folders {
                log::info!(
                    "adding folder: [name='{name}', uri='{uri}']",
                    name = workspace_folder.name,
                    uri = workspace_folder.uri.as_str()
                );
                workspace.add_folder(workspace_folder.uri);
            }
        };
        if let Some(initialization_options) = params.initialization_options {
            log::info!(
                "[PicklsBackend] initialize updating configuration [{initialization_options:?}]",
            );
            update_configuration(&self.client, &self.config, initialization_options);
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
            language_id: _language_id,
            file_contents,
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
        /* let context = InlineAssistTemplateContext { language_id, text };

         let config = self.config.lock();
         let _prompt = create_inline_assist_prompt(&config.ai.inline_assist.template, context)

             .ok_or_else(|| Error {
                 code: TowerLspErrorCode::InvalidParams,
                 message: "Inline assist prompt is not properly configured".into(),
                 data: None,
             })?;
         let Some(_api_key_cmd) = config.ai.openai.as_ref().map(|x| &x.api_key_cmd) else {
             return Err(Error::new("No API key command found for OpenAI").into());
         };
        */
        // let api_key = get_command_output(api_key_cmd)?;
        // let _openai_answer = fetch_inline_assist_response();
        // println!("openai_answer: {:?}", openai_answer);
        Ok(None)
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
            file_contents,
            language_id,
        } = self.get_document(&uri)?;
        let language_config = match self.fetch_language_config(&language_id) {
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
            ).map(|formatted_content| {
                    log::info!(
                        "Formatter {program} succeeded for url '{uri}' [formatted_len={formatted_len}, formatter={program}]",
                        uri = uri.as_str(),
                        formatted_len = formatted_content.len(),
                    );
                    // Create a TextEdit that replaces the whole document
                    edit = Some(TextEdit {
                        range: Range {
                            start: Position::new(0, 0),
                            end: Position::new(u32::MAX, u32::MAX),
                        },
                        new_text: formatted_content.clone(),
                    });
                    formatted_content
                }).context("formatter error")?;
        }

        Ok(edit.map(|edit| vec![edit]))
    }

    fn initialized(&mut self, _: InitializedParams) {
        log::info!(
            "[{site}] initialized called",
            site = self.get_workspace_name()
        );
        self.client
            .log_message(MessageType::INFO, "pickls Server initialized");
    }

    fn did_change_configuration(&mut self, dccp: DidChangeConfigurationParams) {
        if dccp.settings.is_null() {
            return;
        }
        if let serde_json::Value::Object(ref map) = dccp.settings {
            if map.is_empty() {
                return;
            }
        }
        update_configuration(&self.client, &self.config, dccp.settings);
    }

    fn shutdown(&self) -> Result<()> {
        log::info!("[{site}] shutdown called", site = self.get_workspace_name());
        Ok(())
    }

    fn did_close(&mut self, params: DidCloseTextDocumentParams) {
        self.document_storage.remove(&params.text_document.uri);
        log::info!(
            "[{site}] did_close called [params=...]",
            site = self.get_workspace_name()
        );
    }
    fn did_open(&mut self, params: DidOpenTextDocumentParams) {
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
            },
        );
        if let Err(error) = self.run_diagnostics(JobSpec {
            uri: params.text_document.uri,
            version: DocumentVersion(params.text_document.version),
            language_id: params.text_document.language_id,
            text: file_contents,
        }) {
            log::error!("did_open: {error:?}");
        }
    }
    fn did_change(&mut self, mut params: DidChangeTextDocumentParams) {
        log::trace!(
            "[{site}] did_change called [params=...]",
            site = self.get_workspace_name()
        );
        assert!(params.content_changes.len() == 1);
        let file_contents = params.content_changes.remove(0).text;
        let uri = params.text_document.uri;

        let language_id = {
            let mut document_storage_map = self.document_storage;
            let Some(document_storage) = document_storage_map.get_mut(&uri) else {
                self.client.log_message(
                    MessageType::WARNING,
                    format!("no document found for uri {uri}", uri = uri.as_str()),
                );
                return;
            };

            // Update the file contents.
            document_storage.file_contents = file_contents.clone();
            document_storage.language_id.clone()
        };

        if let Err(error) = self.run_diagnostics(JobSpec {
            uri,
            version: DocumentVersion(params.text_document.version),
            language_id,
            text: file_contents,
        }) {
            log::warn!("did_change: {error:?}");
        }
    }
    fn symbol(&mut self, params: WorkspaceSymbolParams) -> Result<Option<Vec<SymbolInformation>>> {
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
    pickls_settings: &PicklsConfig,
    settings: serde_json::Value,
) {
    match serde_json::from_value::<PicklsConfig>(settings) {
        Ok(settings) => {
            *pickls_settings.lock() = settings.clone();
            client.log_message(
                MessageType::INFO,
                format!("configuration changed [config={settings:?}]!"),
            );
        }
        Err(error) => {
            let message = format!("invalid pickls configuration [{error}]");
            log::warn!("{}", message);
            client.log_message(MessageType::WARNING, message);
        }
    }
}

fn setup_logging(base_dirs: &xdg::BaseDirectories, level: log::LevelFilter) -> Result<()> {
    let log_file_path = base_dirs.place_state_file("pickls.log")?;
    simple_logging::log_to_file(log_file_path, level)?;
    Ok(())
}
fn read_config(base_dirs: &xdg::BaseDirectories) -> Option<PicklsConfig> {
    let config_filename = base_dirs.get_config_file(format!("{}.yaml", env!("CARGO_PKG_NAME")));
    log::info!("attempting to read configuration from {config_filename:?}");
    let config = config::parse_config(
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
    let config = read_config(&base_dirs);

    // Initialize the configuration's site name.
    run_server(|client| PicklsBackend::new(client, config.unwrap_or_default()))
}
