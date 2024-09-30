use regex::Captures;

use crate::prelude::*;
use nix::unistd::Pid;

pub async fn run_linter(
    client: &Client,
    linter_config: LintLsLinterConfig,
    file_content: Option<Arc<String>>,
    uri: Url,
    version: i32,
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
                "writing to `{program}`: '{preamble}...'",
                program = linter_config.program,
                preamble = &file_content[..std::cmp::max(file_content.len(), 20)]
            );
            let mut buf: &[u8] = file_content.as_bytes();
            while !buf.is_empty() {
                log::info!("making a write...");
                let written = stdin
                    .write(&buf[..std::cmp::min(buf.len(), 1 << 16)])
                    .await?;
                buf = &buf[written..];
            }
            log::info!("done writing!");
        }
    }

    let client = client.clone();
    let child_pid = Pid::from_raw(child.id().unwrap() as i32);
    tokio::spawn(async move {
        // TODO: maybe box these to enable vtbl-style polymorphism here.
        if let Err(error) = if linter_config.use_stderr {
            ingest_linter_errors(
                uri,
                version,
                client.clone(),
                linter_config,
                BufReader::new(child.stderr.take().expect("Failed to take stderr")),
            )
            .await
        } else {
            ingest_linter_errors(
                uri,
                version,
                client.clone(),
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
    Some(LintLsDiagnostic {
        source: linter_config.program.clone(),
        line,
        start_column,
        end_column,
        description,
    })
}

async fn ingest_linter_errors(
    uri: Url,
    version: i32,
    client: Client,
    linter_config: LintLsLinterConfig,
    child_stdout: impl AsyncBufReadExt + Unpin,
) -> Result<()> {
    let re = Regex::new(&linter_config.pattern).map_err(|e| {
        format!(
            "Invalid regex [pattern={pattern}, error={e}]",
            pattern = linter_config.pattern
        )
    })?;
    // send file as stdin if config wants it.
    let mut reader = child_stdout.lines();
    let mut lsp_diagnostics: Vec<Diagnostic> = Default::default();
    let mut prior_line: Option<String> = None;
    while let Some(line) = reader.next_line().await? {
        log::info!("line: {line}");
        if let Some(caps) = re.captures(&line) {
            log::info!("caps: {caps:?}");
            if let Some(lsp_diagnostic) =
                convert_capture_to_diagnostic(&linter_config, caps, &prior_line)
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
    client
        .publish_diagnostics(uri, lsp_diagnostics, Some(version))
        .await;
    Ok(())
}
