use core::fmt;
use std::collections::VecDeque;

use crate::types::{RespError, RespValue};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RespDecodeError {
    pub message: String,
}

impl RespDecodeError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RespDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RespDecodeError {}

fn read_line(buffer: &[u8]) -> Option<(&[u8], usize)> {
    buffer
        .windows(2)
        .position(|window| window == b"\r\n")
        .map(|pos| (&buffer[..pos], pos + 2))
}

pub fn decode(buffer: &[u8]) -> Result<Option<(RespValue, usize)>, RespDecodeError> {
    if buffer.is_empty() {
        return Ok(None);
    }

    match buffer[0] {
        b'+' => decode_simple_string(buffer),
        b'-' => decode_error(buffer),
        b':' => decode_integer(buffer),
        b'$' => decode_bulk_string(buffer),
        b'*' => decode_array(buffer),
        _ => decode_inline_command(buffer),
    }
}

pub fn decode_all(buffer: &[u8]) -> Result<(Vec<RespValue>, usize), RespDecodeError> {
    let mut messages = Vec::new();
    let mut offset = 0;
    while offset < buffer.len() {
        match decode(&buffer[offset..])? {
            Some((value, consumed)) => {
                messages.push(value);
                offset += consumed;
            }
            None => break,
        }
    }
    Ok((messages, offset))
}

#[derive(Debug, Default, Clone)]
pub struct RespDecoder {
    buffer: Vec<u8>,
    queue: VecDeque<RespValue>,
    error: Option<RespDecodeError>,
}

impl RespDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
        let _ = self.drain();
    }

    pub fn has_message(&self) -> bool {
        !self.queue.is_empty()
    }

    pub fn get_message(&mut self) -> Result<RespValue, RespDecodeError> {
        if let Some(error) = self.error.clone() {
            return Err(error);
        }
        self.queue
            .pop_front()
            .ok_or_else(|| RespDecodeError::new("decoder buffer is empty"))
    }

    pub fn decode_all(&mut self, data: &[u8]) -> Result<Vec<RespValue>, RespDecodeError> {
        self.feed(data);
        if let Some(error) = self.error.clone() {
            return Err(error);
        }
        Ok(self.queue.drain(..).collect())
    }

    fn drain(&mut self) -> Result<(), RespDecodeError> {
        if self.error.is_some() {
            return Ok(());
        }
        loop {
            match decode(&self.buffer) {
                Ok(Some((value, consumed))) => {
                    self.queue.push_back(value);
                    self.buffer.drain(..consumed);
                }
                Ok(None) => break,
                Err(err) => {
                    self.error = Some(err.clone());
                    return Err(err);
                }
            }
        }
        Ok(())
    }
}

fn decode_simple_string(buffer: &[u8]) -> Result<Option<(RespValue, usize)>, RespDecodeError> {
    let Some((line, consumed)) = read_line(&buffer[1..]) else {
        return Ok(None);
    };
    let value = std::str::from_utf8(line)
        .map_err(|_| RespDecodeError::new("invalid UTF-8 in simple string"))?
        .to_string();
    Ok(Some((RespValue::SimpleString(value), consumed + 1)))
}

fn decode_error(buffer: &[u8]) -> Result<Option<(RespValue, usize)>, RespDecodeError> {
    let Some((line, consumed)) = read_line(&buffer[1..]) else {
        return Ok(None);
    };
    let value = std::str::from_utf8(line)
        .map_err(|_| RespDecodeError::new("invalid UTF-8 in error string"))?
        .to_string();
    Ok(Some((RespValue::Error(RespError::new(value)), consumed + 1)))
}

fn decode_integer(buffer: &[u8]) -> Result<Option<(RespValue, usize)>, RespDecodeError> {
    let Some((line, consumed)) = read_line(&buffer[1..]) else {
        return Ok(None);
    };
    let value = std::str::from_utf8(line)
        .map_err(|_| RespDecodeError::new("invalid UTF-8 in integer"))?
        .parse::<i64>()
        .map_err(|_| RespDecodeError::new("invalid RESP integer"))?;
    Ok(Some((RespValue::Integer(value), consumed + 1)))
}

fn decode_bulk_string(buffer: &[u8]) -> Result<Option<(RespValue, usize)>, RespDecodeError> {
    let Some((line, consumed)) = read_line(&buffer[1..]) else {
        return Ok(None);
    };
    let length = std::str::from_utf8(line)
        .map_err(|_| RespDecodeError::new("invalid UTF-8 in bulk string length"))?
        .parse::<isize>()
        .map_err(|_| RespDecodeError::new("invalid RESP bulk string length"))?;

    if length == -1 {
        return Ok(Some((RespValue::BulkString(None), consumed + 1)));
    }
    if length < -1 {
        return Err(RespDecodeError::new("bulk string length cannot be negative"));
    }

    let length = length as usize;
    let body_start = 1 + consumed;
    let body_end = body_start + length;
    let tail_end = body_end + 2;
    if buffer.len() < tail_end {
        return Ok(None);
    }
    if &buffer[body_end..tail_end] != b"\r\n" {
        return Err(RespDecodeError::new(
            "missing trailing CRLF after bulk string body",
        ));
    }
    Ok(Some((
        RespValue::BulkString(Some(buffer[body_start..body_end].to_vec())),
        tail_end,
    )))
}

fn decode_array(buffer: &[u8]) -> Result<Option<(RespValue, usize)>, RespDecodeError> {
    let Some((line, consumed)) = read_line(&buffer[1..]) else {
        return Ok(None);
    };
    let count = std::str::from_utf8(line)
        .map_err(|_| RespDecodeError::new("invalid UTF-8 in array length"))?
        .parse::<isize>()
        .map_err(|_| RespDecodeError::new("invalid RESP array length"))?;

    if count == -1 {
        return Ok(Some((RespValue::Array(None), consumed + 1)));
    }
    if count < -1 {
        return Err(RespDecodeError::new("array length cannot be negative"));
    }

    let mut offset = consumed + 1;
    let mut values = Vec::with_capacity(count as usize);
    for _ in 0..count {
        match decode(&buffer[offset..])? {
            Some((value, used)) => {
                values.push(value);
                offset += used;
            }
            None => return Ok(None),
        }
    }
    Ok(Some((RespValue::Array(Some(values)), offset)))
}

fn decode_inline_command(buffer: &[u8]) -> Result<Option<(RespValue, usize)>, RespDecodeError> {
    let Some((line, consumed)) = read_line(buffer) else {
        return Ok(None);
    };
    let tokens = line
        .split(|byte| byte.is_ascii_whitespace())
        .filter(|token| !token.is_empty())
        .map(|token| RespValue::BulkString(Some(token.to_vec())))
        .collect::<Vec<_>>();
    Ok(Some((RespValue::Array(Some(tokens)), consumed)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{RespError, RespValue};

    fn bulk(value: &str) -> RespValue {
        RespValue::BulkString(Some(value.as_bytes().to_vec()))
    }

    #[test]
    fn decode_covers_all_frame_types_and_edge_cases() {
        assert_eq!(
            decode(b"+OK\r\n").unwrap(),
            Some((RespValue::SimpleString("OK".to_string()), 5))
        );
        assert_eq!(
            decode(b"-ERR boom\r\n").unwrap(),
            Some((RespValue::Error(RespError::new("ERR boom")), 11))
        );
        assert_eq!(
            decode(b":-42\r\n").unwrap(),
            Some((RespValue::Integer(-42), 6))
        );
        assert_eq!(
            decode(b"$-1\r\n").unwrap(),
            Some((RespValue::BulkString(None), 5))
        );
        assert_eq!(
            decode(b"$3\r\nfoo\r\n").unwrap(),
            Some((RespValue::BulkString(Some(b"foo".to_vec())), 9))
        );
        assert_eq!(
            decode(b"*-1\r\n").unwrap(),
            Some((RespValue::Array(None), 5))
        );
        assert_eq!(
            decode(b"*2\r\n+OK\r\n:1\r\n").unwrap(),
            Some((
                RespValue::Array(Some(vec![
                    RespValue::SimpleString("OK".to_string()),
                    RespValue::Integer(1),
                ])),
                13,
            ))
        );
        assert_eq!(
            decode(b"PING  PONG\r\n").unwrap(),
            Some((RespValue::Array(Some(vec![bulk("PING"), bulk("PONG")])), 12))
        );

        assert_eq!(decode(b"+").unwrap(), None);
        assert_eq!(decode(b"$3\r\nfo").unwrap(), None);
        assert_eq!(decode(b"*2\r\n+OK\r\n").unwrap(), None);

        assert!(decode(b"+\xff\r\n").is_err());
        assert!(decode(b"-\xff\r\n").is_err());
        assert!(decode(b":foo\r\n").is_err());
        assert!(decode(b"$\xff\r\n").is_err());
        assert!(decode(b"$-10\r\n").is_err());
        assert!(decode(b"*\xff\r\n").is_err());
        assert!(decode(b"*-10\r\n").is_err());
    }

    #[test]
    fn resp_decoder_accumulates_messages_and_preserves_errors() {
        let mut decoder = RespDecoder::new();
        assert!(!decoder.has_message());
        assert!(decoder.get_message().is_err());

        decoder.feed(b"+OK\r\n");
        assert!(decoder.has_message());
        assert_eq!(
            decoder.get_message().unwrap(),
            RespValue::SimpleString("OK".to_string())
        );
        assert!(!decoder.has_message());

        decoder.feed(b":1\r\n");
        assert!(decoder.has_message());
        assert_eq!(
            decoder.decode_all(b"+PONG\r\n").unwrap(),
            vec![
                RespValue::Integer(1),
                RespValue::SimpleString("PONG".to_string()),
            ]
        );

        let mut error_decoder = RespDecoder::new();
        assert!(error_decoder.decode_all(b"*-10\r\n").is_err());
        assert!(error_decoder.get_message().is_err());
    }
}
