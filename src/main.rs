// src/main.rs
use regex::Regex;
use std::process::Stdio;
use tokio::process::Command;
use tower_lsp::{LspService, Server};

mod config;

#[tokio::main]
async fn main() {
    let (service, socket) = LspService::build(|client| panic!() /*LSP Logic Here */).finish();
    Server::new(tokio::io::stdin(), tokio::io::stdout(), socket)
        .serve(service)
        .await;
}

async fn run_tool(tool: &LintTool, file_path: &str) -> Vec<Diagnostic> {
    let output = Command::new(&tool.path)
        .arg(file_path)
        .stdout(Stdio::piped())
        .spawn()
        .expect(format!("Failed to execute tool [tool.path={}]", tool.path).as_str())
        .wait_with_output()
        .await
        .expect(format!("Failed to read tool output [tool.path={}]", tool.path).as_str());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let re = Regex::new(&tool.pattern).expect(format!("Invalid regex '{}'", tool.pattern));
    stdout
        .lines()
        .filter_map(|line| {
            re.captures(line).map(|caps| Diagnostic {
                filename: caps.get(tool.filename_match).unwrap().as_str().to_string(),
                line: caps.get(tool.line_match).unwrap().as_str().parse().unwrap(),
                col: caps
                    .get(tool.col_match.unwrap_or("0"))
                    .map(|m| m.as_str().parse().unwrap())
                    .unwrap_or(0),
                description: caps
                    .get(tool.description_match)
                    .unwrap()
                    .as_str()
                    .to_string(),
            })
        })
        .collect()
}

use nix::unistd::setsid;
use std::os::unix::process::CommandExt;

fn command_with_new_session(cmd: &mut Command) -> &mut Command {
    cmd.pre_exec(|| setsid().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)))
}
