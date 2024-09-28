use regex::Captures;

use crate::prelude::*;
use nix::unistd::Pid;

pub async fn run_linter(
    client: &Client,
    linter: LintLsLinterConfig,
    uri: Url,
    version: i32,
) -> Result<Pid> {
    let mut child = Command::new(&linter.program)
        .process_group(0)
        .arg(
            uri.to_file_path()
                .map_err(|()| Error::new("invalid file path passed to run_linter".to_string()))?
                .to_str()
                .unwrap(),
        )
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;

    let stdin: tokio::process::ChildStdin = child.stdin.take().expect("Failed to open stdin");
    let stdout: tokio::process::ChildStdout = child.stdout.take().expect("Failed to open stdout");
    let linter = linter.clone();
    let client = client.clone();

    tokio::spawn(async move {
        if let Err(error) = ingest_linter_errors(uri, version, client, linter, stdin, stdout).await
        {
            log::error!("[run_linter/spawn-ingest] error: {error:?}");
        }
    });
    Ok(Pid::from_raw(child.id().unwrap() as i32))
}

fn convert_capture_to_diagnostic(
    linter_config: &LintLsLinterConfig,
    caps: Captures,
    prior_line: &Option<String>,
) -> Option<LintLsDiagnostic> {
    let description: Option<String> = match linter_config.description_match {
        None => None,
        Some(-1) => prior_line.clone(),
        Some(i) if i > 0 => caps.get(i as usize).map(|x| x.as_str().to_string()),
        Some(_) => {
            // TODO: consider logging this broken config.
            None
        }
    };
    // TODO: there are lots of ways this can bail out. Would be nice to eventually handle these and
    // log warnings or traces.
    Some(LintLsDiagnostic {
        line: caps.get(linter_config.line_match)?.as_str().parse().ok()?,
        column: linter_config
            .col_match
            .map(|i| Some(caps.get(i)?.as_str().parse().ok()?))?,
        description,
    })
}

async fn ingest_linter_errors(
    uri: Url,
    version: i32,
    client: Client,
    linter_config: LintLsLinterConfig,
    mut stdin: tokio::process::ChildStdin,
    stdout: tokio::process::ChildStdout,
) -> Result<()> {
    let re = Regex::new(&linter_config.pattern).map_err(|e| {
        format!(
            "Invalid regex [pattern={pattern}, error={e}]",
            pattern = linter_config.pattern
        )
    })?;
    stdin.write_all(b"").await?;
    let mut reader = BufReader::new(stdout).lines();
    let mut lsp_diagnostics: Vec<Diagnostic> = Default::default();
    let mut prior_line: Option<String> = None;
    while let Some(line) = reader.next_line().await? {
        if let Some(caps) = re.captures(&line) {
            if let Some(lsp_diagnostic) =
                convert_capture_to_diagnostic(&linter_config, caps, &prior_line)
            {
                lsp_diagnostics.push(lsp_diagnostic.into());
            }
        }
        prior_line = Some(line);
    }
    client
        .publish_diagnostics(uri, lsp_diagnostics, Some(version))
        .await;
    Ok(())
}
