use std::{
    io::{BufRead, BufReader, Read, Write},
    os::unix::process::CommandExt as _,
};

use nix::unistd::Pid;
use regex::Captures;

use crate::prelude::*;

fn get_root_dir(filename: &str, workspace: &Workspace, root_markers: &[String]) -> Result<String> {
    let starting_path = std::path::PathBuf::from(filename);
    if !root_markers.is_empty() {
        let mut path = starting_path.as_path();
        log::info!("path = {path:?}");
        while path.parent().is_some() {
            path = path.parent().unwrap();
            let path_buf = path.to_path_buf();
            log::trace!("path_buf = {path_buf:?}");
            // Do not allow our search for root markers to go beyond the
            // workspace root (if it starts within it). And stop when we find a
            // root marker.
            if workspace.folders().any(|folder| folder == &path_buf)
                || root_markers
                    .iter()
                    .any(|marker| path_buf.join(marker).exists())
            {
                return Ok(path_buf.to_str().ok_or("invalid root dir")?.to_string());
            }
        }
    }
    let basedir = starting_path
        .parent()
        .ok_or("path has no basedir")?
        .to_str()
        .ok_or("invalid basedir path")?
        .to_string();
    log::info!("no root directory found for file {filename}, using {basedir}");
    Ok(basedir)
}

pub fn run_linter(
    diagnostics_manager: &mut DiagnosticsManager,
    linter_config: PicklsLinterConfig,
    workspace: &Workspace,
    root_markers: &[String],
    max_linter_count: usize,
    file_content: Option<String>,
    uri: Uri,
    version: DocumentVersion,
) -> Result<Pid> {
    let (mut cmd, root_dir) = {
        let filename = uri.path().as_str();

        let mut cmd = Command::new(&linter_config.program);
        let mut args = linter_config.args.clone();
        for arg in args.iter_mut() {
            *arg = arg.replace("$filename", filename);
        }
        let root_dir: String = get_root_dir(filename, workspace, root_markers)?;
        log::info!(
            "running linter {program} with root_dir={root_dir}",
            program = linter_config.program
        );
        cmd.process_group(0)
            .args(args)
            .current_dir(root_dir.clone())
            .stdin(std::process::Stdio::piped());
        if linter_config.use_stderr {
            cmd.stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped());
        } else {
            cmd.stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null());
        }
        (cmd, root_dir)
    };
    log::info!("spawning {cmd:?}...");
    let mut child = cmd.spawn()?;

    let mut stdin: std::process::ChildStdin = child.stdin.take().expect("Failed to open stdin");

    if linter_config.use_stdin {
        if let Some(file_content) = file_content {
            log::info!(
                "writing to `{program}`'s stdin: '{preamble}...'",
                program = linter_config.program,
                preamble = String::from(&file_content[..std::cmp::min(file_content.len(), 20)])
                    .replace("\n", "\\n")
            );
            stdin.write_all(file_content.as_bytes())?;
        }
    }
    drop(stdin);

    let program = linter_config.program.clone();
    let child_pid = Pid::from_raw(child.id() as i32);
    // TODO: maybe box these to enable vtbl-style polymorphism here.
    if let Err(error) = if linter_config.use_stderr {
        ingest_linter_errors(
            uri,
            version,
            diagnostics_manager,
            &root_dir,
            max_linter_count,
            linter_config,
            BufReader::new(child.stderr.take().expect("Failed to take stderr")),
        )
    } else {
        ingest_linter_errors(
            uri,
            version,
            diagnostics_manager,
            &root_dir,
            max_linter_count,
            linter_config,
            BufReader::new(child.stdout.take().expect("Failed to take stdout")),
        )
    } {
        log::error!("[run_linter/spawn-ingest] error: {error:?}");
    }
    child
        .wait()
        .inspect(|status| {
            log::info!(
                "linter program '{program}' exited with status: {status:?} [pid={pid}]",
                pid = child_pid,
            );
        })
        .inspect_err(|err| log::warn!("linter program '{program}' error: {err}",))?;
    Ok(child_pid)
}

fn convert_capture_to_diagnostic(
    absolute_filename: &str,
    linter_config: &PicklsLinterConfig,
    caps: Captures,
    prior_line: &Option<String>,
) -> Option<PicklsDiagnostic> {
    let caps_len = caps.len();
    let description: Option<String> = match linter_config.description_match {
        None => None,
        Some(-1) => prior_line.as_ref().map(|s| s.trim().to_string()),
        Some(i) if i > 0 => {
            if linter_config.line_match >= caps_len {
                log::error!(
                    "invalid description_match in linter configuration of `{program}`: pattern only captures {caps_len} groups but description_match = {i}.",
                    program = linter_config.program
                );
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
    let filename = linter_config
        .filename_match
        .and_then(|i| caps.get(i).map(|x| x.as_str().to_string()))
        .unwrap_or_else(|| absolute_filename.to_string());
    if linter_config.line_match >= caps_len {
        log::error!(
            "invalid line_match in linter configuration of `{program}`: pattern only captures {caps_len} groups but line_match = {line_match}.",
            line_match = linter_config.line_match,
            program = linter_config.program
        );
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
    let severity: Option<PicklsDiagnosticSeverity> = linter_config.severity_match.and_then(|i| {
        Some(PicklsDiagnosticSeverity {
            severity: caps.get(i)?.as_str().to_string(),
        })
    });
    Some(PicklsDiagnostic {
        linter: linter_config.program.clone(),
        filename,
        line,
        start_column,
        end_column,
        severity,
        description,
    })
}

fn ingest_linter_errors(
    uri: Uri,
    version: DocumentVersion,
    diagnostics_manager: &mut DiagnosticsManager,
    root_dir: &str,
    max_linter_count: usize,
    linter_config: PicklsLinterConfig,
    child_stdout: BufReader<impl Read>,
) -> Result<()> {
    let re = Regex::new(&linter_config.pattern).map_err(|e| {
        format!(
            "Invalid regex [pattern={pattern}, error={e}]",
            pattern = linter_config.pattern
        )
    })?;
    let mut lsp_diagnostics: Vec<Diagnostic> = Default::default();
    let mut prior_line: Option<String> = None;
    let root_dir = std::path::PathBuf::from(root_dir);
    let realpath_for_uri = match std::fs::canonicalize(uri.path().as_str()) {
        Ok(path) => path,
        Err(_) => {
            // Fallback to using an absolute path if canonicalization fails. It may fail if the
            // file temporarily doesn't exist due to unlink + move operations that some editors
            // (ie: neovim perform.)
            std::path::absolute(uri.path().as_str())?
        }
    };
    for line in child_stdout.lines() {
        let line = line?;
        // log::info!("line: {line}");
        if let Some(caps) = re.captures(&line) {
            log::trace!("caps: {caps:?}");
            if let Some(lsp_diagnostic) = convert_capture_to_diagnostic(
                uri.path().as_str(),
                &linter_config,
                caps,
                &prior_line,
            ) {
                // log::info!("diagnostic: {lsp_diagnostic:?}");
                let mut path = std::path::PathBuf::from(lsp_diagnostic.filename.clone());
                if path.is_relative() {
                    path = root_dir.join(path);
                }
                let realpath_for_diagnostic = path.canonicalize()?;
                if realpath_for_uri == realpath_for_diagnostic {
                    // Note that this filtering can be avoided in cases that the linter is known to
                    // only lint in the current file. This is intended to filter out diagnostics
                    // from linters that scan multiple files.
                    lsp_diagnostics.push(lsp_diagnostic.into());
                } else {
                    log::warn!(
                        "ignoring diagnostic for {uri} because it is not in the current document [filename={realpath_for_diagnostic:?}]",
                        uri = uri.as_str(),
                    );
                }
            }
        }
        prior_line = Some(line);
    }
    log::info!(
        "publishing diagnostics [linter={linter_name}, count={count}]",
        linter_name = linter_config.program,
        count = lsp_diagnostics.len()
    );

    // TODO: track errors from other documents. For now this is out of reach
    // beacuse we don't have the current version of the other document
    // readily available.
    diagnostics_manager.update_diagnostics(
        uri.clone(),
        // Uri::from_file_path(filename.as_str()).unwrap(),
        linter_config.program.clone(),
        max_linter_count,
        version,
        lsp_diagnostics,
    )
}

pub fn run_formatter(
    formatter_config: &PicklsFormatterConfig,
    workspace: &Workspace,
    root_markers: &[String],
    file_content: String,
    uri: Uri,
) -> Result<String> {
    let mut cmd = {
        let filename = uri.path().as_str();

        let mut cmd = Command::new(&formatter_config.program);
        let mut args = formatter_config.args.clone();
        for arg in args.iter_mut() {
            *arg = arg.replace("$filename", filename);
        }
        let root_dir: String = get_root_dir(filename, workspace, root_markers)?;
        log::info!(
            "running formatter {program} with root_dir={root_dir}",
            program = formatter_config.program
        );
        cmd.process_group(0).args(args).current_dir(root_dir);
        if formatter_config.use_stdin {
            cmd.stdin(std::process::Stdio::piped());
        }
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        cmd
    };

    log::info!("spawning {cmd:?} [stdin={}]", formatter_config.use_stdin);
    let mut child = cmd.spawn()?;
    let mut stdout = child.stdout.take().expect("Failed to open stdout");
    let mut stderr = child.stderr.take().expect("Failed to open stderr");

    if formatter_config.use_stdin {
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        stdin.write_all(file_content.as_bytes())?;
    }

    let mut formatted_content = String::new();
    let mut error_text = String::new();

    match (
        stdout.read_to_string(&mut formatted_content),
        stderr.read_to_string(&mut error_text),
    ) {
        (Ok(stdout_len), Ok(stderr_len)) => {
            log::info!("stdout_len = {stdout_len}, stderr_len = {stderr_len}");
            if formatter_config.stderr_indicates_error && stderr_len != 0 {
                // Writing anything to stderr is considered a formatting failure.
                log::error!(
                    "Failed to format file {uri}: {error_text}",
                    uri = uri.as_str()
                );
                return Err(Error::new("Failed to format file"));
            }
        }
        (Err(err), Err(err2)) => {
            log::error!(
                "Failed to format file {uri}: {err} & {err2}",
                uri = uri.as_str()
            );
            return Err(Error::new("Failed to format file"));
        }
        (Err(err), _) | (_, Err(err)) => {
            log::error!("Failed to format file {uri}: {err}", uri = uri.as_str());
            return Err(Error::new("Failed to format file"));
        }
    };

    let exit_status = child.wait()?;
    if exit_status.success() {
        Ok(formatted_content)
    } else {
        log::error!("Failed to format file {uri}", uri = uri.as_str());
        Err(Error::new("Failed to format file"))
    }
}
