#![allow(unused)]
use std::io::Write;

use crate::prelude::*;

#[derive(Clone)]
pub struct Client {
    stdout: Rc<RefCell<dyn Write>>,
}

impl Client {
    pub fn new(stdout: Rc<RefCell<dyn Write>>) -> Self {
        Self { stdout }
    }
    pub fn log_message(&self, message_type: MessageType, message: impl Into<String>) -> Result<()> {
        self.send_packet(
            "window/LogMessage",
            LogMessageParams {
                typ: message_type,
                message: message.into(),
            },
        )
    }
    pub fn show_message(
        &self,
        message_type: MessageType,
        message: impl Into<String>,
    ) -> Result<()> {
        self.send_packet(
            "window/showMessage",
            ShowMessageParams {
                typ: message_type,
                message: message.into(),
            },
        )
    }
    pub fn send_notification<N: Notification, M: Serialize>(&self, notification: M) -> Result<()> {
        self.send_packet(N::METHOD, notification)
    }
    pub fn publish_diagnostics(
        &self,
        uri: Uri,
        diagnostics: Vec<Diagnostic>,
        version: Option<i32>,
    ) {
        self.send_packet(
            PublishDiagnostics::METHOD,
            PublishDiagnosticsParams {
                uri,
                diagnostics,
                version,
            },
        )
        .unwrap()
    }
    fn send_packet(&self, method: &str, params: impl Serialize) -> Result<()> {
        let json = serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        }))?;
        let mut w = self.stdout.borrow_mut();
        log::trace!("Sending packet length: {}", json.len());
        write!(w, "Content-Length: {}\r\n\r\n{}", json.len(), json)?;
        Ok(w.flush()?)
    }
    pub fn write_response<T: Serialize>(
        &self,
        id: Option<MessageId>,
        result: Result<T>,
    ) -> Result<()> {
        match result {
            Ok(result) => {
                let Some(id) = id else {
                    anyhow::bail!("missing id for response ({})", std::any::type_name::<T>());
                };
                let response_text =
                    serde_json::to_string(&JsonRpcResponse::response(id, result)).unwrap();
                let mut w = self.stdout.borrow_mut();
                log::info!("Sending response length: {}", response_text.len());
                write!(
                    w,
                    "Content-Length: {}\r\n\r\n{}",
                    response_text.len(),
                    response_text
                )?;
                Ok(w.flush()?)
            }
            Err(error) => {
                let id = id.unwrap_or(MessageId::Number(0));
                log::warn!("Sending error response: {error}");
                let response_text =
                    serde_json::to_string(&JsonRpcResponse::error(id, error)).unwrap();
                let mut w = self.stdout.borrow_mut();
                write!(
                    w,
                    "Content-Length: {}\r\n\r\n{}",
                    response_text.len(),
                    response_text
                )?;
                Ok(w.flush()?)
            }
        }
    }
}
