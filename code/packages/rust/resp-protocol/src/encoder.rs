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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{RespArray, RespBulkString, RespError, RespValue};

    #[test]
    fn encodes_scalar_values_and_nested_arrays() {
        assert_eq!(encode_simple_string("OK").unwrap(), b"+OK\r\n".to_vec());
        let err = encode_simple_string("bad\nnews").unwrap_err();
        match &err {
            RespEncodeError::SimpleStringContainsNewline { value } => {
                assert_eq!(value, "bad\nnews");
            }
        }
        assert!(err
            .to_string()
            .contains("simple string must not contain carriage return or newline"));

        assert_eq!(encode_error("ERR boom"), b"-ERR boom\r\n".to_vec());
        assert_eq!(encode_integer(-42), b":-42\r\n".to_vec());
        assert_eq!(encode_bulk_string(RespBulkString::Null), b"$-1\r\n".to_vec());
        assert_eq!(
            encode_bulk_string(RespBulkString::Bytes(b"abc".to_vec())),
            b"$3\r\nabc\r\n".to_vec()
        );

        let encoded = encode_array(RespArray::Values(vec![
            RespValue::from("OK"),
            RespValue::from(7_i64),
            RespValue::from(RespBulkString::Null),
            RespValue::from(RespArray::Values(vec![RespValue::from("nested")])),
        ]))
        .unwrap();
        assert_eq!(encoded, b"*4\r\n+OK\r\n:7\r\n$-1\r\n*1\r\n+nested\r\n".to_vec());

        assert_eq!(encode_array(RespArray::Null).unwrap(), b"*-1\r\n".to_vec());
    }

    #[test]
    fn encode_dispatch_covers_all_resp_variants() {
        assert_eq!(encode(RespValue::from("OK")).unwrap(), b"+OK\r\n".to_vec());
        assert_eq!(
            encode(RespValue::from(RespError::new("ERR boom"))).unwrap(),
            b"-ERR boom\r\n".to_vec()
        );
        assert_eq!(encode(RespValue::from(123_i64)).unwrap(), b":123\r\n".to_vec());
        assert_eq!(
            encode(RespValue::from(RespBulkString::Bytes(b"payload".to_vec()))).unwrap(),
            b"$7\r\npayload\r\n".to_vec()
        );
        assert_eq!(
            encode(RespValue::from(RespArray::Null)).unwrap(),
            b"*-1\r\n".to_vec()
        );
    }
}
