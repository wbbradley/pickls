// src/main.rs
use tower_lsp::{LspService, Server};

mod config;

#[tokio::main]
async fn main() {
    let (service, socket) = LspService::build(|client| panic!() /*LSP Logic Here */).finish();
    Server::new(tokio::io::stdin(), tokio::io::stdout(), socket)
        .serve(service)
        .await;
}
