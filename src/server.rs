use crate::prelude::*;

use lsp_types::request::*;

pub fn run_server<F, T>(f: F) -> Result<()>
where
    F: FnOnce(Client) -> T,
    T: LanguageServer,
{
    let stdin = std::io::stdin();
    let stdout = Rc::new(RefCell::new(std::io::stdout().lock()));
    let client = Client::new(stdout);
    // let mut read_buf = [0i8; 4096];
    let mut backend = f(client.clone());
    log::info!("Server is running");
    for rpc in parse_json_rpc(stdin.lock()) {
        let rpc = rpc.context("Error parsing JSON")?;
        log::info!("Received message: {:#?}", rpc);
        // This is a Request.
        log::info!("Received id: {:?}", rpc.id);
        log::info!("Received method: {}", rpc.method);
        let id = rpc.id.clone();
        match rpc.method.as_str() {
            Initialize::METHOD => {
                client.write_response(id, backend.initialize(rpc.take_params()?)?)?;
            }
            Initialized::METHOD => {
                backend.initialized(rpc.take_params()?);
            }
            SetTrace::METHOD => {
                backend.set_trace(rpc.take_params()?);
            }
            WorkspaceSymbolRequest::METHOD => {
                client.write_response(id, backend.workspace_symbol(rpc.take_params()?)?)?;
            }
            ExecuteCommand::METHOD => {
                client.write_response(id, backend.execute_command(rpc.take_params()?)?)?;
            }
            CodeActionRequest::METHOD => {
                client.write_response(id, backend.code_action(rpc.take_params()?)?)?;
            }
            DidChangeTextDocument::METHOD => {
                backend.did_change(rpc.take_params()?);
            }
            DidChangeConfiguration::METHOD => {
                backend.did_change_configuration(rpc.take_params()?);
            }
            DidOpenTextDocument::METHOD => {
                backend.did_open(rpc.take_params()?);
            }
            DidCloseTextDocument::METHOD => {
                backend.did_close(rpc.take_params()?);
            }
            Formatting::METHOD => {
                client.write_response(id, backend.formatting(rpc.take_params()?)?)?;
            }
            Shutdown::METHOD => {
                client.write_response(id, backend.shutdown()?)?;
                log::info!("Shutting down");
                break;
            }
            _ => {}
        }
    }
    Ok(())
}
