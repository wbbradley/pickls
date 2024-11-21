use crate::prelude::*;

pub struct Client {
    stdout: std::io::Stdout,
}

impl Client {
    pub fn new(stdout: std::io::Stdout) -> Self {
        Self { stdout }
    }
    pub fn log_message(&self, message_type: MessageType, message: impl Into<String>) {
        // let mut stdout = stdout.lock();
        panic!()
    }
    pub fn send_notification<T: Serialize>(&self, notification: T) {
        panic!()
    }
    pub fn publish_diagnostics(
        &self,
        uri: Uri,
        diagnostics: Vec<Diagnostic>,
        version: Option<i32>,
    ) {
        panic!()
    }
}
