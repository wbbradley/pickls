#![allow(unused)]
use crate::prelude::*;
use std::io::BufRead;

pub trait LanguageServer {
    fn code_action(&mut self, params: CodeActionParams) -> Result<Option<CodeActionResponse>>;
    fn did_change(&mut self, params: DidChangeTextDocumentParams);
    fn did_change_configuration(&mut self, dccp: DidChangeConfigurationParams);
    fn did_close(&mut self, params: DidCloseTextDocumentParams);
    fn did_open(&mut self, params: DidOpenTextDocumentParams);
    fn execute_command(&mut self, params: ExecuteCommandParams) -> Result<Option<Value>>;
    fn formatting(&mut self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>>;
    fn initialize(&mut self, params: InitializeParams) -> Result<InitializeResult>;
    fn initialized(&mut self, _: InitializedParams);
    fn shutdown(&self) -> Result<()>;
    fn symbol(&mut self, params: WorkspaceSymbolParams) -> Result<Option<Vec<SymbolInformation>>>;
}

pub struct ParseLsp<R: BufRead> {
    reader: R,
}

pub fn parse_lsp(reader: impl BufRead) -> ParseLsp<impl BufRead> {
    ParseLsp::new(reader)
}

impl<R: BufRead> ParseLsp<R> {
    fn new(reader: R) -> Self {
        Self { reader }
    }
}

impl<R: BufRead> Iterator for ParseLsp<R> {
    type Item = Result<serde_json::Value>;
    fn next(&mut self) -> Option<Result<serde_json::Value>> {
        let mut buf = String::new();
        buf.clear();
        self.reader.read_line(&mut buf).ok()?;
        if buf.is_empty() {
            return None;
        }
        let content_length = if buf.starts_with("Content-Length: ") {
            let content_length = buf
                .trim_start_matches("Content-Length: ")
                .trim_end()
                .parse::<u32>()
                .ok()?;

            log::info!("Got Content-Length: {:?}", content_length);
            content_length
        } else {
            log::error!("Expected Content-Length, got {:?}", buf);
            return None;
        };
        let mut crlf = [0u8; 2];
        self.reader.read_exact(&mut crlf).ok()?;
        if crlf != [13, 10] {
            log::error!("Expected CRLF, got {:?}", crlf);
            panic!()
        }
        let mut buf = vec![0; content_length as usize];
        self.reader.read_exact(&mut buf).ok()?;
        let msg = serde_json::from_slice(&buf).ok()?;
        Some(Ok(msg))
    }
}
