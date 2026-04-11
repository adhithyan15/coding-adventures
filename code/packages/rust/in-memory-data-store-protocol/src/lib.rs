//! RESP-to-engine command frame conversion.

use resp_protocol::{RespError, RespValue};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandFrame {
    pub command: String,
    pub args: Vec<Vec<u8>>,
}

impl CommandFrame {
    pub fn new(command: impl Into<String>, args: Vec<Vec<u8>>) -> Self {
        Self {
            command: command.into(),
            args,
        }
    }

    pub fn from_parts(parts: Vec<Vec<u8>>) -> Option<Self> {
        let (command, args) = parts.split_first()?;
        Some(Self {
            command: ascii_upper(command),
            args: args.to_vec(),
        })
    }
}

pub fn command_frame_from_resp(value: RespValue) -> Option<CommandFrame> {
    match value {
        RespValue::Array(Some(values)) => {
            let mut parts = Vec::with_capacity(values.len());
            for item in values {
                match item {
                    RespValue::BulkString(Some(bytes)) => parts.push(bytes),
                    RespValue::SimpleString(text) => parts.push(text.into_bytes()),
                    RespValue::Integer(n) => parts.push(n.to_string().into_bytes()),
                    _ => return None,
                }
            }
            CommandFrame::from_parts(parts)
        }
        _ => None,
    }
}

pub fn map_resp_decode_error(err: resp_protocol::RespDecodeError) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, err.message)
}

pub fn protocol_error(message: impl Into<String>) -> RespValue {
    RespValue::Error(RespError::new(message))
}

fn ascii_upper(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| byte.to_ascii_uppercase() as char)
        .collect()
}
