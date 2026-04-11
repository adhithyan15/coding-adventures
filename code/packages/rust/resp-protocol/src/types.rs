use core::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RespError {
    pub message: String,
    error_type: String,
    detail: String,
}

impl RespError {
    pub fn new(message: impl Into<String>) -> Self {
        let message = message.into();
        let mut parts = message.splitn(2, ' ');
        let error_type = parts.next().unwrap_or_default().to_string();
        let detail = parts.next().unwrap_or_default().to_string();
        Self {
            message,
            error_type,
            detail,
        }
    }

    pub fn error_type(&self) -> &str {
        &self.error_type
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RespValue {
    SimpleString(String),
    Error(RespError),
    Integer(i64),
    BulkString(Option<Vec<u8>>),
    Array(Option<Vec<RespValue>>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RespBulkString {
    Null,
    Bytes(Vec<u8>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RespArray {
    Null,
    Values(Vec<RespValue>),
}

impl From<&str> for RespValue {
    fn from(value: &str) -> Self {
        Self::SimpleString(value.to_string())
    }
}

impl From<String> for RespValue {
    fn from(value: String) -> Self {
        Self::SimpleString(value)
    }
}

impl From<RespError> for RespValue {
    fn from(value: RespError) -> Self {
        Self::Error(value)
    }
}

impl From<i64> for RespValue {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<i32> for RespValue {
    fn from(value: i32) -> Self {
        Self::Integer(value as i64)
    }
}

impl From<usize> for RespValue {
    fn from(value: usize) -> Self {
        Self::Integer(value as i64)
    }
}

impl From<bool> for RespValue {
    fn from(value: bool) -> Self {
        Self::Integer(if value { 1 } else { 0 })
    }
}

impl From<Vec<u8>> for RespValue {
    fn from(value: Vec<u8>) -> Self {
        Self::BulkString(Some(value))
    }
}

impl From<&[u8]> for RespValue {
    fn from(value: &[u8]) -> Self {
        Self::BulkString(Some(value.to_vec()))
    }
}

impl<const N: usize> From<&[u8; N]> for RespValue {
    fn from(value: &[u8; N]) -> Self {
        Self::BulkString(Some(value.to_vec()))
    }
}

impl<const N: usize> From<[u8; N]> for RespValue {
    fn from(value: [u8; N]) -> Self {
        Self::BulkString(Some(value.to_vec()))
    }
}

impl From<Option<Vec<u8>>> for RespValue {
    fn from(value: Option<Vec<u8>>) -> Self {
        Self::BulkString(value)
    }
}

impl From<Vec<RespValue>> for RespValue {
    fn from(value: Vec<RespValue>) -> Self {
        Self::Array(Some(value))
    }
}

impl From<Option<Vec<RespValue>>> for RespValue {
    fn from(value: Option<Vec<RespValue>>) -> Self {
        Self::Array(value)
    }
}

impl From<RespBulkString> for RespValue {
    fn from(value: RespBulkString) -> Self {
        match value {
            RespBulkString::Null => Self::BulkString(None),
            RespBulkString::Bytes(bytes) => Self::BulkString(Some(bytes)),
        }
    }
}

impl From<RespArray> for RespValue {
    fn from(value: RespArray) -> Self {
        match value {
            RespArray::Null => Self::Array(None),
            RespArray::Values(values) => Self::Array(Some(values)),
        }
    }
}

impl From<Vec<u8>> for RespBulkString {
    fn from(value: Vec<u8>) -> Self {
        Self::Bytes(value)
    }
}

impl From<&[u8]> for RespBulkString {
    fn from(value: &[u8]) -> Self {
        Self::Bytes(value.to_vec())
    }
}

impl<const N: usize> From<&[u8; N]> for RespBulkString {
    fn from(value: &[u8; N]) -> Self {
        Self::Bytes(value.to_vec())
    }
}

impl<const N: usize> From<[u8; N]> for RespBulkString {
    fn from(value: [u8; N]) -> Self {
        Self::Bytes(value.to_vec())
    }
}

impl From<String> for RespBulkString {
    fn from(value: String) -> Self {
        Self::Bytes(value.into_bytes())
    }
}

impl From<&str> for RespBulkString {
    fn from(value: &str) -> Self {
        Self::Bytes(value.as_bytes().to_vec())
    }
}

impl From<Option<Vec<u8>>> for RespBulkString {
    fn from(value: Option<Vec<u8>>) -> Self {
        match value {
            Some(bytes) => Self::Bytes(bytes),
            None => Self::Null,
        }
    }
}

impl From<Vec<RespValue>> for RespArray {
    fn from(value: Vec<RespValue>) -> Self {
        Self::Values(value)
    }
}

impl From<Option<Vec<RespValue>>> for RespArray {
    fn from(value: Option<Vec<RespValue>>) -> Self {
        match value {
            Some(values) => Self::Values(values),
            None => Self::Null,
        }
    }
}

impl fmt::Display for RespError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
