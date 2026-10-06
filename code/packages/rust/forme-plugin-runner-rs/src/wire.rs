use coding_adventures_base64::{decode, encode, STANDARD};
use serde_json::{Map, Number, Value};
use std::collections::BTreeMap;
use std::fmt;

const TAG: &str = "$forme";

#[derive(Clone, Debug, PartialEq)]
pub enum WireValue {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<WireValue>),
    Object(BTreeMap<String, WireValue>),
}

impl WireValue {
    pub fn object<const N: usize>(entries: [(&str, WireValue); N]) -> Self {
        Self::Object(
            entries
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
        )
    }

    pub fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Object(values) => values.get(key),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(value) => value.as_i64(),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn into_object(self) -> Result<BTreeMap<String, Self>, ProtocolError> {
        match self {
            Self::Object(value) => Ok(value),
            _ => Err(ProtocolError::InvalidMessage(
                "JSON-RPC message must be an object".to_owned(),
            )),
        }
    }

    fn encoded_json(&self, budget: usize) -> Result<Value, ProtocolError> {
        if wire_json_size(self)? > budget {
            return Err(ProtocolError::ResourceLimit(
                "wire payload exceeds configured bound".into(),
            ));
        }
        let mut remaining = usize::MAX;
        encode_value(self, &mut remaining)
    }

    pub(crate) fn encoded_size(&self, budget: usize) -> Result<usize, ProtocolError> {
        let size = wire_json_size(self)?;
        if size > budget {
            Err(ProtocolError::ResourceLimit(
                "wire value exceeds configured bound".into(),
            ))
        } else {
            Ok(size)
        }
    }
}

impl From<()> for WireValue {
    fn from(_: ()) -> Self {
        Self::Null
    }
}

impl From<bool> for WireValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<i32> for WireValue {
    fn from(value: i32) -> Self {
        Self::Number(value.into())
    }
}

impl From<i64> for WireValue {
    fn from(value: i64) -> Self {
        Self::Number(value.into())
    }
}

impl From<u64> for WireValue {
    fn from(value: u64) -> Self {
        Self::Number(value.into())
    }
}

impl From<usize> for WireValue {
    fn from(value: usize) -> Self {
        Self::Number((value as u64).into())
    }
}

impl From<String> for WireValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for WireValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<Vec<u8>> for WireValue {
    fn from(value: Vec<u8>) -> Self {
        Self::Bytes(value)
    }
}

impl From<Vec<WireValue>> for WireValue {
    fn from(value: Vec<WireValue>) -> Self {
        Self::Array(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProtocolError {
    InvalidFrame(String),
    InvalidMessage(String),
    ResourceLimit(String),
    Closed(String),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFrame(message)
            | Self::InvalidMessage(message)
            | Self::ResourceLimit(message)
            | Self::Closed(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ProtocolError {}

pub fn encode_frame(value: &WireValue, max_frame_bytes: usize) -> Result<Vec<u8>, ProtocolError> {
    let json = value.encoded_json(max_frame_bytes)?;
    let payload = serde_json::to_vec(&json).map_err(|_| {
        ProtocolError::InvalidMessage("wire payload is not JSON serializable".into())
    })?;
    if payload.len() > max_frame_bytes {
        return Err(ProtocolError::ResourceLimit(
            "wire payload exceeds configured bound".into(),
        ));
    }
    let header = format!("Content-Length: {}\r\n\r\n", payload.len());
    let mut frame = Vec::with_capacity(header.len() + payload.len());
    frame.extend_from_slice(header.as_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

fn consume(remaining: &mut usize, amount: usize) -> Result<(), ProtocolError> {
    *remaining = remaining.checked_sub(amount).ok_or_else(|| {
        ProtocolError::ResourceLimit("wire payload exceeds configured bound".into())
    })?;
    Ok(())
}

fn checked_sum(left: usize, right: usize) -> Result<usize, ProtocolError> {
    left.checked_add(right)
        .ok_or_else(|| ProtocolError::ResourceLimit("wire payload exceeds configured bound".into()))
}

fn json_string_size(value: &str) -> Result<usize, ProtocolError> {
    let mut size = 2_usize;
    for byte in value.bytes() {
        let encoded = match byte {
            b'"' | b'\\' | 0x08 | 0x09 | 0x0a | 0x0c | 0x0d => 2,
            0x00..=0x1f => 6,
            _ => 1,
        };
        size = checked_sum(size, encoded)?;
    }
    Ok(size)
}

fn wire_json_size(value: &WireValue) -> Result<usize, ProtocolError> {
    match value {
        WireValue::Null => Ok(4),
        WireValue::Bool(true) => Ok(4),
        WireValue::Bool(false) => Ok(5),
        WireValue::Number(value) => Ok(value.to_string().len()),
        WireValue::String(value) => json_string_size(value),
        WireValue::Bytes(value) => {
            let encoded = value
                .len()
                .checked_add(2)
                .and_then(|length| length.checked_div(3))
                .and_then(|length| length.checked_mul(4))
                .ok_or_else(|| {
                    ProtocolError::ResourceLimit("wire payload exceeds configured bound".into())
                })?;
            checked_sum(r#"{"$forme":"bytes","base64":""}"#.len(), encoded)
        }
        WireValue::Array(values) => {
            let mut size = checked_sum(2, values.len().saturating_sub(1))?;
            for value in values {
                size = checked_sum(size, wire_json_size(value)?)?;
            }
            Ok(size)
        }
        WireValue::Object(values) if values.contains_key(TAG) => {
            let mut size = r#"{"$forme":"escaped-object","entries":[]}"#.len();
            size = checked_sum(size, values.len().saturating_sub(1))?;
            for (key, value) in values {
                let entry = checked_sum(
                    checked_sum(json_string_size(key)?, wire_json_size(value)?)?,
                    3,
                )?;
                size = checked_sum(size, entry)?;
            }
            Ok(size)
        }
        WireValue::Object(values) => {
            let mut size = checked_sum(2, values.len().saturating_sub(1))?;
            for (key, value) in values {
                size = checked_sum(size, json_string_size(key)?)?;
                size = checked_sum(size, 1)?;
                size = checked_sum(size, wire_json_size(value)?)?;
            }
            Ok(size)
        }
    }
}

fn encode_value(value: &WireValue, remaining: &mut usize) -> Result<Value, ProtocolError> {
    consume(remaining, 1)?;
    match value {
        WireValue::Null => Ok(Value::Null),
        WireValue::Bool(value) => Ok(Value::Bool(*value)),
        WireValue::Number(value) => Ok(Value::Number(value.clone())),
        WireValue::String(value) => {
            consume(remaining, value.len())?;
            Ok(Value::String(value.clone()))
        }
        WireValue::Bytes(value) => {
            let encoded_len = value
                .len()
                .checked_add(2)
                .and_then(|length| length.checked_div(3))
                .and_then(|length| length.checked_mul(4))
                .ok_or_else(|| {
                    ProtocolError::ResourceLimit("wire payload exceeds configured bound".into())
                })?;
            consume(remaining, encoded_len)?;
            let encoded = encode(value, &STANDARD);
            Ok(serde_json::json!({TAG: "bytes", "base64": encoded}))
        }
        WireValue::Array(values) => {
            let mut result = Vec::with_capacity(values.len());
            for value in values {
                result.push(encode_value(value, remaining)?);
            }
            Ok(Value::Array(result))
        }
        WireValue::Object(values) if values.contains_key(TAG) => {
            let mut entries = Vec::with_capacity(values.len());
            for (key, value) in values {
                consume(remaining, key.len())?;
                entries.push(Value::Array(vec![
                    Value::String(key.clone()),
                    encode_value(value, remaining)?,
                ]));
            }
            Ok(serde_json::json!({TAG: "escaped-object", "entries": entries}))
        }
        WireValue::Object(values) => {
            let mut result = Map::new();
            for (key, value) in values {
                consume(remaining, key.len())?;
                result.insert(key.clone(), encode_value(value, remaining)?);
            }
            Ok(Value::Object(result))
        }
    }
}

pub fn decode_wire_value(value: Value) -> Result<WireValue, ProtocolError> {
    match value {
        Value::Null => Ok(WireValue::Null),
        Value::Bool(value) => Ok(WireValue::Bool(value)),
        Value::Number(value) => Ok(WireValue::Number(value)),
        Value::String(value) => Ok(WireValue::String(value)),
        Value::Array(values) => values
            .into_iter()
            .map(decode_wire_value)
            .collect::<Result<Vec<_>, _>>()
            .map(WireValue::Array),
        Value::Object(mut values) => {
            if let Some(tag) = values.remove(TAG) {
                let tag = tag.as_str().ok_or_else(|| {
                    ProtocolError::InvalidMessage("reserved wire envelope is malformed".into())
                })?;
                return match tag {
                    "bytes" if values.len() == 1 => {
                        let encoded = values
                            .remove("base64")
                            .and_then(|value| value.as_str().map(str::to_owned))
                            .ok_or_else(|| {
                                ProtocolError::InvalidMessage("byte envelope is malformed".into())
                            })?;
                        let bytes = decode(&encoded, &STANDARD).map_err(|_| {
                            ProtocolError::InvalidMessage("byte envelope is malformed".into())
                        })?;
                        if encode(&bytes, &STANDARD) != encoded {
                            return Err(ProtocolError::InvalidMessage(
                                "byte envelope is not canonical".into(),
                            ));
                        }
                        Ok(WireValue::Bytes(bytes))
                    }
                    "escaped-object" if values.len() == 1 => {
                        let entries = values
                            .remove("entries")
                            .and_then(|value| value.as_array().cloned())
                            .ok_or_else(|| {
                                ProtocolError::InvalidMessage(
                                    "escaped object envelope is malformed".into(),
                                )
                            })?;
                        let mut result = BTreeMap::new();
                        for entry in entries {
                            let mut pair = entry.as_array().cloned().ok_or_else(|| {
                                ProtocolError::InvalidMessage(
                                    "escaped object entry is malformed".into(),
                                )
                            })?;
                            if pair.len() != 2 {
                                return Err(ProtocolError::InvalidMessage(
                                    "escaped object entry is malformed".into(),
                                ));
                            }
                            let decoded = decode_wire_value(pair.pop().unwrap())?;
                            let key = pair
                                .pop()
                                .and_then(|value| value.as_str().map(str::to_owned))
                                .ok_or_else(|| {
                                    ProtocolError::InvalidMessage(
                                        "escaped object key is malformed".into(),
                                    )
                                })?;
                            if result.insert(key, decoded).is_some() {
                                return Err(ProtocolError::InvalidMessage(
                                    "escaped object contains a duplicate key".into(),
                                ));
                            }
                        }
                        Ok(WireValue::Object(result))
                    }
                    _ => Err(ProtocolError::InvalidMessage(
                        "reserved wire envelope is malformed".into(),
                    )),
                };
            }
            values
                .into_iter()
                .map(|(key, value)| Ok((key, decode_wire_value(value)?)))
                .collect::<Result<BTreeMap<_, _>, _>>()
                .map(WireValue::Object)
        }
    }
}

pub struct FrameDecoder {
    max_frame_bytes: usize,
    max_header_bytes: usize,
    max_buffer_bytes: usize,
    buffer: Vec<u8>,
    expected: Option<usize>,
}

impl FrameDecoder {
    pub fn new(max_frame_bytes: usize, max_header_bytes: usize) -> Result<Self, ProtocolError> {
        if max_frame_bytes == 0 || max_header_bytes == 0 {
            return Err(ProtocolError::ResourceLimit(
                "wire bounds must be positive".into(),
            ));
        }
        let max_buffer_bytes = max_header_bytes
            .checked_add(4)
            .and_then(|value| value.checked_add(max_frame_bytes))
            .ok_or_else(|| {
                ProtocolError::ResourceLimit("wire bounds exceed platform capacity".into())
            })?;
        Ok(Self {
            max_frame_bytes,
            max_header_bytes,
            max_buffer_bytes,
            buffer: Vec::with_capacity(max_header_bytes.min(4096)),
            expected: None,
        })
    }

    pub fn push(&mut self, chunk: &[u8]) -> Result<Vec<WireValue>, ProtocolError> {
        if self.buffer.len().saturating_add(chunk.len()) > self.max_buffer_bytes {
            return Err(ProtocolError::ResourceLimit(
                "wire input exceeds configured buffer bound".into(),
            ));
        }
        self.buffer.extend_from_slice(chunk);
        let mut messages = Vec::new();
        loop {
            if self.expected.is_none() {
                let end = find_subsequence(&self.buffer, b"\r\n\r\n");
                let Some(end) = end else {
                    if self.buffer.len() > self.max_header_bytes {
                        return Err(ProtocolError::ResourceLimit(
                            "wire header exceeds configured bound".into(),
                        ));
                    }
                    break;
                };
                if end > self.max_header_bytes {
                    return Err(ProtocolError::ResourceLimit(
                        "wire header exceeds configured bound".into(),
                    ));
                }
                let header = std::str::from_utf8(&self.buffer[..end])
                    .map_err(|_| ProtocolError::InvalidFrame("wire header is not ASCII".into()))?;
                let mut lengths = Vec::new();
                for line in header.split("\r\n") {
                    if let Some((key, value)) = line.split_once(':') {
                        if key.eq_ignore_ascii_case("content-length") {
                            lengths.push(value.trim_matches([' ', '\t']));
                        }
                    }
                }
                if lengths.len() != 1
                    || lengths[0].is_empty()
                    || (lengths[0].len() > 1 && lengths[0].starts_with('0'))
                    || !lengths[0].bytes().all(|byte| byte.is_ascii_digit())
                {
                    return Err(ProtocolError::InvalidFrame(
                        "frame requires exactly one canonical Content-Length".into(),
                    ));
                }
                let expected = lengths[0]
                    .parse::<usize>()
                    .map_err(|_| ProtocolError::InvalidFrame("Content-Length is invalid".into()))?;
                if expected > self.max_frame_bytes {
                    return Err(ProtocolError::ResourceLimit(
                        "wire payload exceeds configured bound".into(),
                    ));
                }
                self.buffer.drain(..end + 4);
                self.expected = Some(expected);
            }
            let expected = self.expected.unwrap();
            if self.buffer.len() < expected {
                break;
            }
            let payload: Vec<u8> = self.buffer.drain(..expected).collect();
            self.expected = None;
            let decoded: Value = serde_json::from_slice(&payload).map_err(|_| {
                ProtocolError::InvalidFrame("wire payload is not valid JSON".into())
            })?;
            messages.push(decode_wire_value(decoded)?);
        }
        Ok(messages)
    }

    pub(crate) fn remaining_capacity(&self) -> usize {
        self.max_buffer_bytes - self.buffer.len()
    }

    pub fn finish(&self) -> Result<(), ProtocolError> {
        if self.buffer.is_empty() && self.expected.is_none() {
            Ok(())
        } else {
            Err(ProtocolError::InvalidFrame(
                "wire ended in the middle of a frame".into(),
            ))
        }
    }
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preflight_size_matches_serialized_wire_snapshot() {
        let values = [
            WireValue::String("quote: \"; control: \u{0001}; snowman: ☃".into()),
            WireValue::Bytes(vec![0, 1, 2, 253, 254, 255]),
            WireValue::object([
                (TAG, "literal".into()),
                (
                    "nested",
                    WireValue::Array(vec![true.into(), WireValue::Null]),
                ),
            ]),
        ];
        for value in values {
            let json = value.encoded_json(usize::MAX).unwrap();
            let serialized = serde_json::to_vec(&json).unwrap();
            assert_eq!(wire_json_size(&value).unwrap(), serialized.len());
        }
    }
}
