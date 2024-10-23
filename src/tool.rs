use regex::Captures;

use crate::prelude::*;
use nix::unistd::Pid;

pub async fn run_linter(
    diagnostics_manager: DiagnosticsManager,
    linter_config: LintLsLinterConfig,
    file_content: Option<Arc<String>>,
    uri: Url,
    version: DocumentVersion,
) -> Result<Pid> {
    let mut cmd = {
        let filename = uri
            .to_file_path()
            .map_err(|()| "invalid file path passed to run_linter")?;
        let filename = filename.to_str().unwrap();

        let mut cmd = Command::new(&linter_config.program);
        let mut args = linter_config.args.clone();
        for arg in args.iter_mut() {
            *arg = arg.replace("$filename", filename);
        }
        cmd.process_group(0)
            .args(args)
            .stdin(std::process::Stdio::piped());
        if linter_config.use_stderr {
            cmd.stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped());
        } else {
            cmd.stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null());
        }
        cmd
    };
    log::info!("spawning {cmd:?}...");
    let mut child = cmd.spawn()?;

    let mut stdin: tokio::process::ChildStdin = child.stdin.take().expect("Failed to open stdin");

    if linter_config.use_stdin {
        if let Some(file_content) = file_content {
            log::info!(
                "writing to `{program}`'s stdin: '{preamble}...'",
                program = linter_config.program,
                preamble = String::from(&file_content[..std::cmp::min(file_content.len(), 20)])
                    .replace("\n", "\\n")
            );
            stdin.write_all(file_content.as_bytes()).await?;
        }
    }

    let child_pid = Pid::from_raw(child.id().unwrap() as i32);
    tokio::spawn(async move {
        // TODO: maybe box these to enable vtbl-style polymorphism here.
        if let Err(error) = if linter_config.use_stderr {
            ingest_linter_errors(
                uri,
                version,
                diagnostics_manager.clone(),
                linter_config,
                BufReader::new(child.stderr.take().expect("Failed to take stderr")),
            )
            .await
        } else {
            ingest_linter_errors(
                uri,
                version,
                diagnostics_manager.clone(),
                linter_config,
                BufReader::new(child.stdout.take().expect("Failed to take stdout")),
            )
            .await
        } {
            log::error!("[run_linter/spawn-ingest] error: {error:?}");
        }
    });
    Ok(child_pid)
}

fn convert_capture_to_diagnostic(
    absolute_filename: &str,
    linter_config: &LintLsLinterConfig,
    caps: Captures,
    prior_line: &Option<String>,
) -> Option<LintLsDiagnostic> {
    let caps_len = caps.len();
    let description: Option<String> = match linter_config.description_match {
        None => None,
        Some(-1) => prior_line.as_ref().map(|s| s.trim().to_string()),
        Some(i) if i > 0 => {
            if linter_config.line_match >= caps_len {
                log::error!(
                    "invalid description_match in linter configuration of `{program}`: pattern only captures {caps_len} groups but description_match = {i}.",
                    program = linter_config.program);
                return None;
            }
            caps.get(i as usize).map(|x| x.as_str().to_string())
        }
        Some(value) => {
            log::error!(
                "invalid description_match in linter configuration of `{program}`: description_match={value}",
                program = linter_config.program
            );
            None
        }
    };
    if linter_config.line_match >= caps_len {
        log::error!(
            "invalid line_match in linter configuration of `{program}`: pattern only captures {caps_len} groups but line_match = {line_match}.",
            line_match = linter_config.line_match,
            program = linter_config.program);
        return None;
    }
    let line: u32 = caps
        .get(linter_config.line_match)
        .unwrap()
        .as_str()
        .parse()
        .ok()?;
    let start_column = linter_config
        .start_col_match
        .and_then(|i| caps.get(i)?.as_str().parse().ok());
    let end_column = linter_config
        .end_col_match
        .and_then(|i| caps.get(i)?.as_str().parse().ok());
    let severity: Option<LintLsDiagnosticSeverity> = linter_config.severity_match.and_then(|i| {
        Some(LintLsDiagnosticSeverity {
            severity: caps.get(i)?.as_str().to_string(),
        })
    });
    Some(LintLsDiagnostic {
        filename: absolute_filename.to_string(),
        source: linter_config.program.clone(),
        line,
        start_column,
        end_column,
        severity,
        description,
    })
}

async fn ingest_linter_errors(
    uri: Url,
    version: DocumentVersion,
    mut diagnostics_manager: DiagnosticsManager,
    linter_config: LintLsLinterConfig,
    child_stdout: impl AsyncBufReadExt + Unpin,
) -> Result<()> {
    let re = Regex::new(&linter_config.pattern).map_err(|e| {
        format!(
            "Invalid regex [pattern={pattern}, error={e}]",
            pattern = linter_config.pattern
        )
    })?;
    let mut reader = child_stdout.lines();
    let mut lsp_diagnostics: Vec<Diagnostic> = Default::default();
    let mut prior_line: Option<String> = None;
    while let Some(line) = reader.next_line().await? {
        log::info!("line: {line}");
        if let Some(caps) = re.captures(&line) {
            log::info!("caps: {caps:?}");
            if let Some(lsp_diagnostic) =
                convert_capture_to_diagnostic(uri.path(), &linter_config, caps, &prior_line)
            {
                lsp_diagnostics.push(lsp_diagnostic.into());
            }
        }
        prior_line = Some(line);
    }
    log::info!(
        "publishing diagnostics [count={count}]",
        count = lsp_diagnostics.len()
    );

    diagnostics_manager
        .update_diagnostics(uri, linter_config.program.clone(), version, lsp_diagnostics)
        .await;
    Ok(())
}
