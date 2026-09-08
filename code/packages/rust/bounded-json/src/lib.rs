//! Zero-dependency bounded RFC 8259 parsing for hostile protocol inputs.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::fmt::{self, Display, Formatter};

/// Default maximum number of nested arrays and objects.
pub const DEFAULT_MAX_DEPTH: usize = 128;

/// One JSON number after exact lexical validation.
#[derive(Clone, Debug, PartialEq)]
pub enum JsonNumber {
    /// Integer representable as an `i64`.
    Integer(i64),
    /// Finite number requiring an exponent or fractional representation.
    Float(f64),
}

/// One parsed JSON value.
#[derive(Clone, Debug, PartialEq)]
pub enum JsonValue {
    /// Source-ordered object members; duplicate names are intentionally retained.
    Object(Vec<(String, JsonValue)>),
    /// Ordered array members.
    Array(Vec<JsonValue>),
    /// Decoded Unicode string.
    String(String),
    /// Exact integer or finite floating-point number.
    Number(JsonNumber),
    /// Boolean value.
    Bool(bool),
    /// Null value.
    Null,
}

/// Closed parse failure class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsonErrorKind {
    /// Input ended before the current value was complete.
    UnexpectedEnd,
    /// The next byte was not valid in the current grammar position.
    UnexpectedToken,
    /// A string contained an invalid escape, control byte, or surrogate.
    InvalidString,
    /// A number violated RFC 8259 grammar or exceeded supported representation.
    InvalidNumber,
    /// Array/object nesting exceeded the caller's bound.
    DepthLimit,
    /// Non-whitespace input remained after the first value.
    TrailingData,
}

/// Input-free parse error with a byte offset and closed class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JsonError {
    kind: JsonErrorKind,
    offset: usize,
}

impl JsonError {
    /// Return the closed failure class.
    pub const fn kind(&self) -> JsonErrorKind {
        self.kind
    }

    /// Return the byte offset at which parsing stopped.
    pub const fn offset(&self) -> usize {
        self.offset
    }
}

impl Display for JsonError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "json parse {:?} at byte {}",
            self.kind, self.offset
        )
    }
}

impl std::error::Error for JsonError {}

/// Parse one JSON value with [`DEFAULT_MAX_DEPTH`].
pub fn parse(input: &str) -> Result<JsonValue, JsonError> {
    parse_with_depth_limit(input, DEFAULT_MAX_DEPTH)
}

/// Parse one JSON value while rejecting arrays/objects deeper than `max_depth`.
///
/// `max_depth == 0` permits scalar roots and rejects every container.
pub fn parse_with_depth_limit(input: &str, max_depth: usize) -> Result<JsonValue, JsonError> {
    let mut parser = Parser {
        input,
        bytes: input.as_bytes(),
        position: 0,
        max_depth,
    };
    parser.skip_whitespace();
    let value = parser.parse_value(0)?;
    parser.skip_whitespace();
    if parser.position != parser.bytes.len() {
        return Err(parser.error(JsonErrorKind::TrailingData));
    }
    Ok(value)
}

struct Parser<'a> {
    input: &'a str,
    bytes: &'a [u8],
    position: usize,
    max_depth: usize,
}

impl Parser<'_> {
    fn parse_value(&mut self, depth: usize) -> Result<JsonValue, JsonError> {
        self.skip_whitespace();
        match self.peek() {
            Some(b'{') => self.parse_object(depth),
            Some(b'[') => self.parse_array(depth),
            Some(b'"') => self.parse_string().map(JsonValue::String),
            Some(b't') => {
                self.consume_literal(b"true")?;
                Ok(JsonValue::Bool(true))
            }
            Some(b'f') => {
                self.consume_literal(b"false")?;
                Ok(JsonValue::Bool(false))
            }
            Some(b'n') => {
                self.consume_literal(b"null")?;
                Ok(JsonValue::Null)
            }
            Some(b'-' | b'0'..=b'9') => self.parse_number().map(JsonValue::Number),
            Some(_) => Err(self.error(JsonErrorKind::UnexpectedToken)),
            None => Err(self.error(JsonErrorKind::UnexpectedEnd)),
        }
    }

    fn parse_object(&mut self, depth: usize) -> Result<JsonValue, JsonError> {
        self.enter_container(depth)?;
        self.position += 1;
        self.skip_whitespace();
        let mut members = Vec::new();
        if self.consume_if(b'}') {
            return Ok(JsonValue::Object(members));
        }
        loop {
            if self.peek() != Some(b'"') {
                return Err(self.error(self.end_or(JsonErrorKind::UnexpectedToken)));
            }
            let name = self.parse_string()?;
            self.skip_whitespace();
            self.expect(b':')?;
            self.position += 1;
            let value = self.parse_value(depth + 1)?;
            members.push((name, value));
            self.skip_whitespace();
            if self.consume_if(b'}') {
                break;
            }
            self.expect(b',')?;
            self.position += 1;
            self.skip_whitespace();
        }
        Ok(JsonValue::Object(members))
    }

    fn parse_array(&mut self, depth: usize) -> Result<JsonValue, JsonError> {
        self.enter_container(depth)?;
        self.position += 1;
        self.skip_whitespace();
        let mut values = Vec::new();
        if self.consume_if(b']') {
            return Ok(JsonValue::Array(values));
        }
        loop {
            values.push(self.parse_value(depth + 1)?);
            self.skip_whitespace();
            if self.consume_if(b']') {
                break;
            }
            self.expect(b',')?;
            self.position += 1;
            self.skip_whitespace();
        }
        Ok(JsonValue::Array(values))
    }

    fn enter_container(&self, depth: usize) -> Result<(), JsonError> {
        if depth >= self.max_depth {
            Err(self.error(JsonErrorKind::DepthLimit))
        } else {
            Ok(())
        }
    }

    fn parse_string(&mut self) -> Result<String, JsonError> {
        self.expect(b'"')?;
        self.position += 1;
        let mut output = String::new();
        let mut segment_start = self.position;
        loop {
            let Some(byte) = self.peek() else {
                return Err(self.error(JsonErrorKind::UnexpectedEnd));
            };
            match byte {
                b'"' => {
                    output.push_str(&self.input[segment_start..self.position]);
                    self.position += 1;
                    return Ok(output);
                }
                b'\\' => {
                    output.push_str(&self.input[segment_start..self.position]);
                    self.position += 1;
                    self.parse_escape(&mut output)?;
                    segment_start = self.position;
                }
                0x00..=0x1f => return Err(self.error(JsonErrorKind::InvalidString)),
                _ => self.position += 1,
            }
        }
    }

    fn parse_escape(&mut self, output: &mut String) -> Result<(), JsonError> {
        let Some(byte) = self.peek() else {
            return Err(self.error(JsonErrorKind::UnexpectedEnd));
        };
        self.position += 1;
        match byte {
            b'"' => output.push('"'),
            b'\\' => output.push('\\'),
            b'/' => output.push('/'),
            b'b' => output.push('\u{0008}'),
            b'f' => output.push('\u{000c}'),
            b'n' => output.push('\n'),
            b'r' => output.push('\r'),
            b't' => output.push('\t'),
            b'u' => {
                let high = self.parse_hex_quad()?;
                let code_point = if (0xd800..=0xdbff).contains(&high) {
                    if self.bytes.get(self.position..self.position + 2) != Some(b"\\u") {
                        return Err(self.error(JsonErrorKind::InvalidString));
                    }
                    self.position += 2;
                    let low = self.parse_hex_quad()?;
                    if !(0xdc00..=0xdfff).contains(&low) {
                        return Err(self.error(JsonErrorKind::InvalidString));
                    }
                    0x10000 + ((u32::from(high) - 0xd800) << 10) + (u32::from(low) - 0xdc00)
                } else if (0xdc00..=0xdfff).contains(&high) {
                    return Err(self.error(JsonErrorKind::InvalidString));
                } else {
                    u32::from(high)
                };
                output.push(
                    char::from_u32(code_point)
                        .ok_or_else(|| self.error(JsonErrorKind::InvalidString))?,
                );
            }
            _ => return Err(self.error(JsonErrorKind::InvalidString)),
        }
        Ok(())
    }

    fn parse_hex_quad(&mut self) -> Result<u16, JsonError> {
        let end = self
            .position
            .checked_add(4)
            .ok_or_else(|| self.error(JsonErrorKind::InvalidString))?;
        let Some(bytes) = self.bytes.get(self.position..end) else {
            return Err(self.error(JsonErrorKind::UnexpectedEnd));
        };
        let mut value = 0_u16;
        for byte in bytes {
            value = value
                .checked_mul(16)
                .and_then(|current| hex_value(*byte).map(|digit| current + u16::from(digit)))
                .ok_or_else(|| self.error(JsonErrorKind::InvalidString))?;
        }
        self.position = end;
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<JsonNumber, JsonError> {
        let start = self.position;
        if self.consume_if(b'-') && self.peek().is_none() {
            return Err(self.error(JsonErrorKind::UnexpectedEnd));
        }
        match self.peek() {
            Some(b'0') => {
                self.position += 1;
                if matches!(self.peek(), Some(b'0'..=b'9')) {
                    return Err(self.error(JsonErrorKind::InvalidNumber));
                }
            }
            Some(b'1'..=b'9') => self.consume_digits(),
            Some(_) => return Err(self.error(JsonErrorKind::InvalidNumber)),
            None => return Err(self.error(JsonErrorKind::UnexpectedEnd)),
        }
        let mut fractional = false;
        if self.consume_if(b'.') {
            fractional = true;
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.error(self.end_or(JsonErrorKind::InvalidNumber)));
            }
            self.consume_digits();
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            fractional = true;
            self.position += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.position += 1;
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.error(self.end_or(JsonErrorKind::InvalidNumber)));
            }
            self.consume_digits();
        }
        let source = &self.input[start..self.position];
        if fractional {
            let value = source
                .parse::<f64>()
                .map_err(|_| self.error(JsonErrorKind::InvalidNumber))?;
            if !value.is_finite() {
                return Err(self.error(JsonErrorKind::InvalidNumber));
            }
            Ok(JsonNumber::Float(value))
        } else {
            source
                .parse::<i64>()
                .map(JsonNumber::Integer)
                .map_err(|_| self.error(JsonErrorKind::InvalidNumber))
        }
    }

    fn consume_digits(&mut self) {
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.position += 1;
        }
    }

    fn consume_literal(&mut self, literal: &[u8]) -> Result<(), JsonError> {
        let end = self
            .position
            .checked_add(literal.len())
            .ok_or_else(|| self.error(JsonErrorKind::UnexpectedEnd))?;
        match self.bytes.get(self.position..end) {
            Some(found) if found == literal => {
                self.position = end;
                Ok(())
            }
            Some(_) => Err(self.error(JsonErrorKind::UnexpectedToken)),
            None => Err(self.error(JsonErrorKind::UnexpectedEnd)),
        }
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.position += 1;
        }
    }

    fn expect(&self, expected: u8) -> Result<(), JsonError> {
        match self.peek() {
            Some(found) if found == expected => Ok(()),
            Some(_) => Err(self.error(JsonErrorKind::UnexpectedToken)),
            None => Err(self.error(JsonErrorKind::UnexpectedEnd)),
        }
    }

    fn consume_if(&mut self, expected: u8) -> bool {
        if self.peek() == Some(expected) {
            self.position += 1;
            true
        } else {
            false
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }

    fn end_or(&self, other: JsonErrorKind) -> JsonErrorKind {
        if self.position == self.bytes.len() {
            JsonErrorKind::UnexpectedEnd
        } else {
            other
        }
    }

    const fn error(&self, kind: JsonErrorKind) -> JsonError {
        JsonError {
            kind,
            offset: self.position,
        }
    }
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_value_shape_and_preserves_duplicate_members() {
        assert_eq!(
            parse(r#"{"a":1,"a":-2.5e+3,"v":[true,false,null,"x"]}"#).unwrap(),
            JsonValue::Object(vec![
                ("a".to_owned(), JsonValue::Number(JsonNumber::Integer(1))),
                (
                    "a".to_owned(),
                    JsonValue::Number(JsonNumber::Float(-2_500.0)),
                ),
                (
                    "v".to_owned(),
                    JsonValue::Array(vec![
                        JsonValue::Bool(true),
                        JsonValue::Bool(false),
                        JsonValue::Null,
                        JsonValue::String("x".to_owned()),
                    ]),
                ),
            ])
        );
    }

    #[test]
    fn strings_decode_unicode_and_reject_invalid_sequences() {
        assert_eq!(
            parse(r#""plain café \\ \" \/ \b \f \n \r \t \u03bb \ud83d\ude80""#).unwrap(),
            JsonValue::String("plain café \\ \" / \u{0008} \u{000c} \n \r \t λ 🚀".to_owned())
        );
        for invalid in [r#""\x""#, r#""\ud800""#, r#""\udc00""#, "\"line\nfeed\""] {
            assert!(matches!(
                parse(invalid),
                Err(JsonError {
                    kind: JsonErrorKind::InvalidString | JsonErrorKind::UnexpectedEnd,
                    ..
                })
            ));
        }
    }

    #[test]
    fn exact_number_grammar_rejects_extensions_and_overflow() {
        for invalid in ["01", "-", "1.", "1e", "+1", "NaN", "1e9999"] {
            assert!(parse(invalid).is_err(), "accepted {invalid}");
        }
        assert_eq!(
            parse("-0").unwrap(),
            JsonValue::Number(JsonNumber::Integer(0))
        );
    }

    #[test]
    fn depth_is_rejected_before_unbounded_recursion() {
        assert_eq!(
            parse_with_depth_limit("[[[0]]]", 2).unwrap_err().kind(),
            JsonErrorKind::DepthLimit
        );
        assert!(parse_with_depth_limit("[[0]]", 2).is_ok());
        let hostile = format!("{}0{}", "[".repeat(100_000), "]".repeat(100_000));
        assert_eq!(
            parse_with_depth_limit(&hostile, 64).unwrap_err().kind(),
            JsonErrorKind::DepthLimit
        );
    }

    #[test]
    fn malformed_structure_and_trailing_data_are_closed() {
        for invalid in ["", "[", "[1,]", "{\"a\"}", "{\"a\":1,}", "true false"] {
            assert!(parse(invalid).is_err(), "accepted {invalid}");
        }
        assert_eq!(
            parse("true false").unwrap_err().kind(),
            JsonErrorKind::TrailingData
        );
    }

    #[test]
    fn errors_never_copy_source_text() {
        let secret = "{\"access_token\":\"top-secret\",}";
        let error = parse(secret).unwrap_err();
        let rendered = format!("{error:?} {error}");
        assert!(!rendered.contains("top-secret"));
        assert!(error.offset() < secret.len());
    }
}
