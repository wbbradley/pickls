use crate::error::{Context as _, Error, Result};
use handlebars::Handlebars;
use lsp_types::Range;
use serde::Serialize;
use std::process;
pub use sysinfo::{Pid, System};

pub fn fetch_parent_process_info() -> String {
    let mut system = System::new_all();
    system.refresh_all();

    let current_pid = process::id();
    if let Some(current_process) = system.process(Pid::from(current_pid as usize)) {
        if let Some(parent_pid) = current_process.parent() {
            if let Some(parent_process) = system.process(parent_pid) {
                return format!(
                    "[name={name:?}, user_id={user_id:?}]",
                    name = parent_process.name().to_string_lossy().into_owned(),
                    user_id = parent_process.user_id(),
                );
            }
        }
    }
    "<unknown parent process>".to_string()
}

/// Returns a slice of the source code based on the given range.
pub fn slice_range(source: &str, range: Range) -> String {
    let start_line = range.start.line as i64;
    let start_character = range.start.character as i64;
    let end_line = range.end.line as i64;
    let end_character = range.end.character as i64;
    let mut line: i64 = 0;
    let mut character: i64 = 0;
    let mut slice = String::new();
    for ch in source.encode_utf16() {
        if (line > start_line || (line == start_line && character >= start_character))
            && ((line < end_line) || (line == end_line && character < end_character))
        {
            // TODO: PERF reverse the encoding math to find the original slice offset to avoid all
            // this allocation.
            let s = char::from_u32(ch.into()).unwrap_or(' ');
            slice.push(s);
        }
        if ch == 10 {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }
    }
    slice
}

#[test]
fn test_slice_range() {
    use lsp_types::Position;
    assert_eq!(
        "bcdef\ndh",
        slice_range(
            "abcdef\ndhi",
            Range {
                start: Position {
                    line: 0,
                    character: 1
                },
                end: Position {
                    line: 1,
                    character: 2
                }
            }
        )
    );
    let source = "fn main() {\n    println!(\"Hello, world!\");\n}\n";
    let range = Range {
        start: Position {
            line: 1,
            character: 4,
        },
        end: Position {
            line: 1,
            character: 12,
        },
    };
    assert_eq!(slice_range(source, range), "println!");
}

#[allow(dead_code)]
pub fn outdent_text(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let min_indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

    lines
        .iter()
        .map(|line| {
            if line.len() >= min_indent {
                &line[min_indent..]
            } else {
                line
            }
        })
        .collect::<Vec<&str>>()
        .join("\n")
}

pub async fn get_command_output(cmd: Vec<String>) -> Result<String> {
    use tokio::process::Command;
    let output = Command::new(&cmd[0])
        .args(&cmd[1..])
        .output()
        .await
        .context("Failed to run command")?;
    if !output.status.success() {
        return Err(Error::new(format!(
            "Command failed with status: {status:?}",
            status = output.status
        )));
    }
    String::from_utf8(output.stdout).context("Failed to read stdout as utf-8")
}

pub fn render_template<T: Serialize>(template: &str, context: T) -> Result<String> {
    let mut reg = Handlebars::new();
    // Avoid all escaping.
    reg.register_escape_fn(|x| x.to_string());
    Ok(reg.render_template(template, &context)?)
}
