#![allow(unused)]
use crate::prelude::*;
use std::io::Write;

#[derive(Clone)]
pub struct Client {
    stdout: Rc<RefCell<dyn Write>>,
}

impl Client {
    pub fn new(stdout: Rc<RefCell<dyn Write>>) -> Self {
        Self { stdout }
    }
    pub fn log_message(&self, _message_type: MessageType, _message: impl Into<String>) {
        // let mut stdout = stdout.lock();
        panic!()
    }
    pub fn send_notification<T: Serialize>(&self, _notification: T) {
        panic!()
    }
    pub fn publish_diagnostics(
        &self,
        _uri: Uri,
        _diagnostics: Vec<Diagnostic>,
        _version: Option<i32>,
    ) {
        panic!()
    }
    pub fn write_response<T: Serialize>(&self, id: Option<MessageId>, result: T) -> Result<()> {
        let Some(id) = id else {
            return Err(Error::new(format!(
                "missing id for response ({})",
                std::any::type_name::<T>()
            )));
        };
        let response_text = serde_json::to_string(&JsonRpcResponse::response(id, result)).unwrap();
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
