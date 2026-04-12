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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resp_error_splits_message_into_type_and_detail() {
        let error = RespError::new("ERR boom");
        assert_eq!(error.message, "ERR boom");
        assert_eq!(error.error_type(), "ERR");
        assert_eq!(error.detail(), "boom");
        assert_eq!(error.to_string(), "ERR boom");

        let standalone = RespError::new("ERR");
        assert_eq!(standalone.error_type(), "ERR");
        assert_eq!(standalone.detail(), "");
    }

    #[test]
    fn resp_value_and_container_conversions_cover_all_variants() {
        assert_eq!(RespValue::from("ok"), RespValue::SimpleString("ok".to_string()));
        assert_eq!(
            RespValue::from(String::from("ok")),
            RespValue::SimpleString("ok".to_string())
        );

        let error = RespError::new("ERR boom");
        assert_eq!(RespValue::from(error.clone()), RespValue::Error(error));
        assert_eq!(RespValue::from(7_i64), RespValue::Integer(7));
        assert_eq!(RespValue::from(7_i32), RespValue::Integer(7));
        assert_eq!(RespValue::from(7_usize), RespValue::Integer(7));
        assert_eq!(RespValue::from(true), RespValue::Integer(1));
        assert_eq!(RespValue::from(false), RespValue::Integer(0));
        assert_eq!(
            RespValue::from(vec![1_u8, 2, 3]),
            RespValue::BulkString(Some(vec![1, 2, 3]))
        );
        assert_eq!(
            RespValue::from(&b"abc"[..]),
            RespValue::BulkString(Some(b"abc".to_vec()))
        );
        let byte_array = b"abc";
        assert_eq!(
            RespValue::from(byte_array),
            RespValue::BulkString(Some(b"abc".to_vec()))
        );
        assert_eq!(
            RespValue::from(*b"abc"),
            RespValue::BulkString(Some(b"abc".to_vec()))
        );
        assert_eq!(
            RespValue::from(Some(vec![4_u8, 5, 6])),
            RespValue::BulkString(Some(vec![4, 5, 6]))
        );
        assert_eq!(RespValue::from(None::<Vec<u8>>), RespValue::BulkString(None));

        let array_values = vec![RespValue::from("nested"), RespValue::from(9_i64)];
        assert_eq!(
            RespValue::from(array_values.clone()),
            RespValue::Array(Some(array_values.clone()))
        );
        assert_eq!(
            RespValue::from(Some(array_values.clone())),
            RespValue::Array(Some(array_values.clone()))
        );
        assert_eq!(RespValue::from(None::<Vec<RespValue>>), RespValue::Array(None));
        assert_eq!(
            RespValue::from(RespBulkString::Null),
            RespValue::BulkString(None)
        );
        assert_eq!(
            RespValue::from(RespBulkString::Bytes(b"bytes".to_vec())),
            RespValue::BulkString(Some(b"bytes".to_vec()))
        );
        assert_eq!(RespValue::from(RespArray::Null), RespValue::Array(None));
        assert_eq!(
            RespValue::from(RespArray::Values(array_values.clone())),
            RespValue::Array(Some(array_values.clone()))
        );

        assert_eq!(RespBulkString::from(None::<Vec<u8>>), RespBulkString::Null);
        assert_eq!(
            RespBulkString::from("abc"),
            RespBulkString::Bytes(b"abc".to_vec())
        );
        assert_eq!(
            RespBulkString::from(vec![1_u8, 2, 3]),
            RespBulkString::Bytes(vec![1, 2, 3])
        );
        assert_eq!(
            RespBulkString::from(byte_array),
            RespBulkString::Bytes(b"abc".to_vec())
        );
        assert_eq!(
            RespBulkString::from(*b"abc"),
            RespBulkString::Bytes(b"abc".to_vec())
        );
        assert_eq!(
            RespBulkString::from(Some(vec![1_u8, 2, 3])),
            RespBulkString::Bytes(vec![1, 2, 3])
        );

        assert_eq!(RespArray::from(None::<Vec<RespValue>>), RespArray::Null);
        assert_eq!(
            RespArray::from(vec![RespValue::from("x")]),
            RespArray::Values(vec![RespValue::from("x")])
        );
        assert_eq!(
            RespArray::from(Some(vec![RespValue::from(1_i64)])),
            RespArray::Values(vec![RespValue::from(1_i64)])
        );
    }
}
