#![allow(unused)]
use crate::prelude::*;

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
