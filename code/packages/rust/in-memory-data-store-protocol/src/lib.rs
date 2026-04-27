//! In-memory data store protocol intermediate representation.

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

fn ascii_upper(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| byte.to_ascii_uppercase() as char)
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EngineResponse {
    SimpleString(String),
    Error(String),
    Integer(i64),
    BulkString(Option<Vec<u8>>),
    Array(Option<Vec<EngineResponse>>),
}

impl EngineResponse {
    pub fn simple_string(s: impl Into<String>) -> Self {
        Self::SimpleString(s.into())
    }

    pub fn error(e: impl Into<String>) -> Self {
        Self::Error(e.into())
    }

    pub fn ok() -> Self {
        Self::SimpleString("OK".to_string())
    }

    pub fn null() -> Self {
        Self::BulkString(None)
    }

    pub fn zero() -> Self {
        Self::Integer(0)
    }

    pub fn one() -> Self {
        Self::Integer(1)
    }
}
