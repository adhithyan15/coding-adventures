use core::fmt;

use crate::types::{RespArray, RespBulkString, RespValue};

const CRLF: &[u8] = b"\r\n";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RespEncodeError {
    SimpleStringContainsNewline { value: String },
}

impl fmt::Display for RespEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SimpleStringContainsNewline { value } => write!(
                f,
                "simple string must not contain carriage return or newline: {value:?}"
            ),
        }
    }
}

impl std::error::Error for RespEncodeError {}

pub fn encode_simple_string(value: impl AsRef<str>) -> Result<Vec<u8>, RespEncodeError> {
    let value = value.as_ref();
    if value.contains('\r') || value.contains('\n') {
        return Err(RespEncodeError::SimpleStringContainsNewline {
            value: value.to_string(),
        });
    }
    let mut out = Vec::with_capacity(value.len() + 3);
    out.push(b'+');
    out.extend_from_slice(value.as_bytes());
    out.extend_from_slice(CRLF);
    Ok(out)
}

pub fn encode_error(value: impl AsRef<str>) -> Vec<u8> {
    let value = value.as_ref();
    let mut out = Vec::with_capacity(value.len() + 3);
    out.push(b'-');
    out.extend_from_slice(value.as_bytes());
    out.extend_from_slice(CRLF);
    out
}

pub fn encode_integer(value: i64) -> Vec<u8> {
    let bytes = value.to_string().into_bytes();
    let mut out = Vec::with_capacity(bytes.len() + 3);
    out.push(b':');
    out.extend_from_slice(&bytes);
    out.extend_from_slice(CRLF);
    out
}

pub fn encode_bulk_string(value: impl Into<RespBulkString>) -> Vec<u8> {
    match value.into() {
        RespBulkString::Null => b"$-1\r\n".to_vec(),
        RespBulkString::Bytes(bytes) => {
            let header = format!("${}\r\n", bytes.len()).into_bytes();
            let mut out = Vec::with_capacity(header.len() + bytes.len() + 2);
            out.extend_from_slice(&header);
            out.extend_from_slice(&bytes);
            out.extend_from_slice(CRLF);
            out
        }
    }
}

pub fn encode_array(value: impl Into<RespArray>) -> Result<Vec<u8>, RespEncodeError> {
    match value.into() {
        RespArray::Null => Ok(b"*-1\r\n".to_vec()),
        RespArray::Values(values) => {
            let mut out = format!("*{}\r\n", values.len()).into_bytes();
            for item in values {
                out.extend_from_slice(&encode(item)?);
            }
            Ok(out)
        }
    }
}

pub fn encode(value: impl Into<RespValue>) -> Result<Vec<u8>, RespEncodeError> {
    match value.into() {
        RespValue::SimpleString(value) => encode_simple_string(value),
        RespValue::Error(err) => Ok(encode_error(err.message)),
        RespValue::Integer(value) => Ok(encode_integer(value)),
        RespValue::BulkString(value) => Ok(encode_bulk_string(value)),
        RespValue::Array(value) => encode_array(value),
    }
}
