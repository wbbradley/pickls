#![allow(unused)]
use crate::prelude::*;
use serde::de::{self, Deserialize, Deserializer, Visitor};
use std::io::BufRead;

pub trait LanguageServer {
    fn code_action(&mut self, params: CodeActionParams) -> Result<Option<CodeActionResponse>>;
    fn did_change(&mut self, params: DidChangeTextDocumentParams) -> Result<()>;
    fn did_change_configuration(&mut self, dccp: DidChangeConfigurationParams) -> Result<()>;
    fn did_close(&mut self, params: DidCloseTextDocumentParams) -> Result<()>;
    fn did_open(&mut self, params: DidOpenTextDocumentParams) -> Result<()>;
    // fn will_save(&mut self, params: WillSaveTextDocumentParams) -> Result<()>;
    // fn did_save(&mut self, params: DidSaveTextDocumentParams) -> Result<()>;
    fn execute_command(&mut self, params: ExecuteCommandParams) -> Result<Option<Value>>;
    fn formatting(&mut self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>>;
    fn initialize(&mut self, params: InitializeParams) -> Result<InitializeResult>;
    fn initialized(&mut self, _: InitializedParams) -> Result<()>;
    fn shutdown(&self) -> Result<()>;
    fn set_trace(&mut self, params: SetTraceParams) {}
    fn workspace_symbol(
        &mut self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>>;
    // fn exit(&self);
}

pub struct ParseJsonRpc<R: BufRead> {
    reader: R,
}

pub fn parse_json_rpc(reader: impl BufRead) -> ParseJsonRpc<impl BufRead> {
    ParseJsonRpc::new(reader)
}

impl<R: BufRead> ParseJsonRpc<R> {
    fn new(reader: R) -> Self {
        Self { reader }
    }
}

/// JSON-RPC 2.0 message ID
#[derive(Clone, Debug)]
pub enum MessageId {
    Number(i64),
    String(String),
    Null,
}

impl Serialize for MessageId {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            MessageId::Number(n) => serializer.serialize_i64(*n),
            MessageId::String(s) => serializer.serialize_str(s),
            MessageId::Null => serializer.serialize_unit(),
        }
    }
}

#[test]
fn test_message_id_json() {
    assert_eq!(serde_json::to_string(&MessageId::Number(42)).unwrap(), "42");
    assert_eq!(serde_json::to_string(&MessageId::Null).unwrap(), "null");
    assert_eq!(
        serde_json::to_string(&MessageId::String("a".to_string())).unwrap(),
        "\"a\""
    );
}

impl<'de> Deserialize<'de> for MessageId {
    fn deserialize<D>(deserializer: D) -> std::result::Result<MessageId, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MessageIdVisitor;

        impl<'de> Visitor<'de> for MessageIdVisitor {
            type Value = MessageId;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a number, a string, or null")
            }

            fn visit_i64<E>(self, value: i64) -> std::result::Result<MessageId, E> {
                Ok(MessageId::Number(value))
            }

            fn visit_u64<E>(self, value: u64) -> std::result::Result<MessageId, E>
            where
                E: de::Error,
            {
                Ok(MessageId::Number(value as i64))
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<MessageId, E>
            where
                E: de::Error,
            {
                Ok(MessageId::String(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> std::result::Result<MessageId, E> {
                Ok(MessageId::String(value))
            }

            fn visit_unit<E>(self) -> std::result::Result<MessageId, E> {
                Ok(MessageId::Null)
            }
        }

        deserializer.deserialize_any(MessageIdVisitor)
    }
}

#[derive(Serialize)]
pub struct JsonRpcResponse<T> {
    pub jsonrpc: &'static str,
    pub id: MessageId,
    pub result: T,
}

impl<T: Serialize> JsonRpcResponse<T> {
    pub fn response(id: MessageId, result: T) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result,
        }
    }
}

/// JSON-RPC 2.0 message
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpc {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<MessageId>,
    pub method: String,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

impl JsonRpc {
    pub fn take_params<T: DeserializeOwned>(self) -> Result<T> {
        Ok(serde_json::from_value(self.params.unwrap())?)
    }
}

impl<R: BufRead> Iterator for ParseJsonRpc<R> {
    type Item = Result<JsonRpc>;
    fn next(&mut self) -> Option<Result<JsonRpc>> {
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
