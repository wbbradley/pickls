use crate::prelude::*;
use std::io::{BufRead, BufReader, Write};

use lsp_types::request::*;

pub fn run_server<'a, F, T: LanguageServer>(f: F) -> Result<()>
where
    F: FnOnce(&Client) -> T,
{
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let client = Client::new(stdin.lock());
    let mut buf = String::new();
    let backend = f(&client);
    log::info!("Server is running");

    for line in BufReader::new(stdin.lock()).lines() {
        let line = line.unwrap();
        if line.is_empty() {
            continue;
        }

        buf.push_str(&line);

        if let Some(pos) = buf.find("\r\n\r\n") {
            let (_, json) = buf.split_at(pos + 4);
            let msg: serde_json::Value = serde_json::from_str(json).unwrap();
            buf.clear();

            if let Some(id) = msg.get("id").and_then(|i| i.as_i64()) {
                if let Some(method) = msg.get("method").and_then(|m| m.as_str()) {
                    match method {
                        Initialize::METHOD => {
                            let params: InitializeParams =
                                serde_json::from_value(msg["params"].clone()).unwrap();
                            let result = backend.initialize(params);
                            let response = json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": result,
                            });
                            write_response(&mut stdout, &response);
                        }
                        _ => {}
                    }
                }
            } else if let Some(method) = msg.get("method").and_then(|m| m.as_str()) {
                if method == DidOpenTextDocument::METHOD {
                    let params: DidOpenTextDocumentParams =
                        serde_json::from_value(msg["params"].clone()).unwrap();
                    backend.handle_did_open_document(params);
                }
            }
        }
    }
}

fn write_response<W: Write>(writer: &mut W, response: &serde_json::Value) -> Result<()> {
    let response_text = serde_json::to_string(response).unwrap();
    write!(
        writer,
        "Content-Length: {}\r\n\r\n{}",
        response_text.len(),
        response_text
    )?;
    Ok(writer.flush()?)
}
