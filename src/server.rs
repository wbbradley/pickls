use crate::prelude::*;
use std::io::{BufRead, BufReader};

use lsp_types::request::*;

pub fn run_server<F, T>(f: F) -> Result<()>
where
    F: FnOnce(Client) -> T,
    T: LanguageServer,
{
    let stdin = std::io::stdin();
    let stdout = Rc::new(RefCell::new(std::io::stdout().lock()));
    let client = Client::new(stdout);
    let mut buf = String::new();
    let mut backend = f(client.clone());
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
                    log::info!("Received method: {}", method);
                    match method {
                        Initialize::METHOD => {
                            let f = msg.get("params").cloned().unwrap();
                            let params: InitializeParams = serde_json::from_value(f).unwrap();
                            let result = backend.initialize(params)?;
                            let response = json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": result,
                            });
                            client.write_response(&response);
                        }
                        _ => {}
                    }
                }
            } else if let Some(method) = msg.get("method").and_then(|m| m.as_str()) {
                if method == DidOpenTextDocument::METHOD {
                    let params: DidOpenTextDocumentParams =
                        serde_json::from_value(msg["params"].clone()).unwrap();
                    backend.did_open(params);
                }
            }
        }
    }
    Ok(())
}
