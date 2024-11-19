use std::process;
pub use sysinfo::{Pid, System};
use tower_lsp::lsp_types::Range;

pub async fn fetch_parent_process_info() -> String {
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
    use tower_lsp::lsp_types::Position;
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
        start: tower_lsp::lsp_types::Position {
            line: 1,
            character: 4,
        },
        end: tower_lsp::lsp_types::Position {
            line: 1,
            character: 12,
        },
    };
    assert_eq!(slice_range(source, range), "println!");
}
