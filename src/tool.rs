use regex::Captures;

use crate::prelude::*;
use nix::unistd::Pid;

pub async fn run_tool(client: &Client, tool: &LintTool, uri: Url, version: i32) -> Result<Pid> {
    let mut child = Command::new(&tool.program)
        .process_group(0)
        .arg(
            uri.to_file_path()
                .map_err(|()| Error::new("invalid file path passed to run_tool".to_string()))?
                .to_str()
                .unwrap(),
        )
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;

    let stdin: tokio::process::ChildStdin = child.stdin.take().expect("Failed to open stdin");
    let stdout: tokio::process::ChildStdout = child.stdout.take().expect("Failed to open stdout");
    let tool = tool.clone();
    let client = client.clone();

    tokio::spawn(async move {
        if let Err(error) = ingest_tool_errors(uri, version, client, tool, stdin, stdout).await {
            log::error!("[run_tool/spawn-ingest] error: {error:?}");
        }
    });
    Ok(Pid::from_raw(child.id().unwrap() as i32))
}

fn convert_capture_to_diagnostic(tool: &LintTool, caps: Captures) -> Option<LintLsDiagnostic> {
    Some(LintLsDiagnostic {
        line: caps.get(tool.line_match)?.as_str().parse().unwrap(),
        description: tool
            .description_match
            .map(|i| Some(caps.get(i)?.as_str().to_string()))?,
    })
}

async fn ingest_tool_errors(
    uri: Url,
    version: i32,
    client: Client,
    tool: LintTool,
    mut stdin: tokio::process::ChildStdin,
    stdout: tokio::process::ChildStdout,
) -> Result<()> {
    let re = Regex::new(&tool.pattern).map_err(|e| {
        format!(
            "Invalid regex [pattern={pattern}, error={e}]",
            pattern = tool.pattern
        )
    })?;
    stdin.write_all(b"").await?;
    let mut reader = BufReader::new(stdout).lines();
    let mut lsp_diagnostics: Vec<Diagnostic> = Default::default();
    while let Some(line) = reader.next_line().await? {
        if let Some(caps) = re.captures(&line) {
            if let Some(lsp_diagnostic) = convert_capture_to_diagnostic(&tool, caps) {
                lsp_diagnostics.push(lsp_diagnostic.into());
            }
        }
    }
    client
        .publish_diagnostics(uri, lsp_diagnostics, Some(version))
        .await;
    Ok(())
}
