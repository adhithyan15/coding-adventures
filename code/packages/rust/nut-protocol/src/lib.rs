//! Strict bounded Network UPS Tools read-only protocol framing.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fmt;

pub const VERSION: &str = "0.1.0";
pub const DEFAULT_PORT: u16 = 3493;
pub const MAX_LINE_BYTES: usize = 4_096;
pub const MAX_RESPONSE_BYTES: usize = 65_536;
pub const MAX_VARIABLES: usize = 64;
pub const MAX_NAME_BYTES: usize = 128;
pub const MAX_VALUE_BYTES: usize = 1_024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NutProtocolError {
    InvalidName { field: &'static str },
    ResponseTooLarge,
    LineTooLong,
    InvalidUtf8,
    Malformed(String),
    ServerError(String),
    UpsMismatch { expected: String, actual: String },
    DuplicateVariable(String),
    TooManyVariables,
}

impl fmt::Display for NutProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName { field } => write!(formatter, "invalid NUT {field}"),
            Self::ResponseTooLarge => write!(formatter, "NUT response exceeds the byte limit"),
            Self::LineTooLong => write!(formatter, "NUT response line exceeds the byte limit"),
            Self::InvalidUtf8 => write!(formatter, "NUT response is not valid UTF-8"),
            Self::Malformed(message) => write!(formatter, "malformed NUT response: {message}"),
            Self::ServerError(message) => write!(formatter, "NUT server error: {message}"),
            Self::UpsMismatch { expected, actual } => write!(
                formatter,
                "NUT UPS name mismatch: expected `{expected}`, got `{actual}`"
            ),
            Self::DuplicateVariable(name) => {
                write!(formatter, "duplicate NUT variable `{name}`")
            }
            Self::TooManyVariables => write!(formatter, "NUT response has too many variables"),
        }
    }
}

impl std::error::Error for NutProtocolError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NutVariable {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListVarResponse {
    pub ups_name: String,
    pub variables: Vec<NutVariable>,
}

pub fn validate_ups_name(value: &str) -> Result<(), NutProtocolError> {
    validate_token(value, "UPS name")
}

pub fn validate_variable_name(value: &str) -> Result<(), NutProtocolError> {
    validate_token(value, "variable name")
}

pub fn encode_list_var_request(ups_name: &str) -> Result<Vec<u8>, NutProtocolError> {
    validate_ups_name(ups_name)?;
    Ok(format!("LIST VAR {ups_name}\n").into_bytes())
}

pub fn list_var_end_line(ups_name: &str) -> Result<String, NutProtocolError> {
    validate_ups_name(ups_name)?;
    Ok(format!("END LIST VAR {ups_name}"))
}

pub fn decode_list_var_response(
    bytes: &[u8],
    expected_ups_name: &str,
) -> Result<ListVarResponse, NutProtocolError> {
    validate_ups_name(expected_ups_name)?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(NutProtocolError::ResponseTooLarge);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| NutProtocolError::InvalidUtf8)?;
    if !text.ends_with('\n') {
        return Err(malformed("response must end with a newline"));
    }
    let mut lines = text
        .split_terminator('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line));
    let first = lines
        .next()
        .ok_or_else(|| malformed("missing list header"))?;
    check_line(first)?;
    if let Some(error) = first.strip_prefix("ERR ") {
        return Err(NutProtocolError::ServerError(error.to_string()));
    }
    let header_name = first
        .strip_prefix("BEGIN LIST VAR ")
        .ok_or_else(|| malformed("expected BEGIN LIST VAR header"))?;
    require_ups(expected_ups_name, header_name)?;

    let mut variables = Vec::new();
    let mut names = BTreeSet::new();
    let mut ended = false;
    for line in lines {
        check_line(line)?;
        if ended {
            return Err(malformed("records follow the list terminator"));
        }
        if let Some(name) = line.strip_prefix("END LIST VAR ") {
            require_ups(expected_ups_name, name)?;
            ended = true;
            continue;
        }
        if let Some(error) = line.strip_prefix("ERR ") {
            return Err(NutProtocolError::ServerError(error.to_string()));
        }
        if variables.len() == MAX_VARIABLES {
            return Err(NutProtocolError::TooManyVariables);
        }
        let remainder = line
            .strip_prefix("VAR ")
            .ok_or_else(|| malformed("expected VAR record or list terminator"))?;
        let (ups_name, remainder) = split_once_space(remainder)?;
        require_ups(expected_ups_name, ups_name)?;
        let (name, encoded_value) = split_once_space(remainder)?;
        validate_variable_name(name)?;
        if !names.insert(name.to_string()) {
            return Err(NutProtocolError::DuplicateVariable(name.to_string()));
        }
        variables.push(NutVariable {
            name: name.to_string(),
            value: decode_quoted(encoded_value)?,
        });
    }
    if !ended {
        return Err(malformed("missing END LIST VAR terminator"));
    }
    Ok(ListVarResponse {
        ups_name: expected_ups_name.to_string(),
        variables,
    })
}

fn validate_token(value: &str, field: &'static str) -> Result<(), NutProtocolError> {
    if value.is_empty()
        || value.len() > MAX_NAME_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(NutProtocolError::InvalidName { field });
    }
    Ok(())
}

fn require_ups(expected: &str, actual: &str) -> Result<(), NutProtocolError> {
    validate_ups_name(actual)?;
    if expected != actual {
        return Err(NutProtocolError::UpsMismatch {
            expected: expected.to_string(),
            actual: actual.to_string(),
        });
    }
    Ok(())
}

fn split_once_space(value: &str) -> Result<(&str, &str), NutProtocolError> {
    let (left, right) = value
        .split_once(' ')
        .ok_or_else(|| malformed("record has too few fields"))?;
    if left.is_empty() || right.is_empty() || right.starts_with(' ') {
        return Err(malformed("record fields must use one separating space"));
    }
    Ok((left, right))
}

fn decode_quoted(value: &str) -> Result<String, NutProtocolError> {
    if value.len() < 2 || !value.starts_with('"') || !value.ends_with('"') {
        return Err(malformed("variable value must be quoted"));
    }
    let mut decoded = String::new();
    let mut escaped = false;
    for character in value[1..value.len() - 1].chars() {
        if escaped {
            match character {
                '"' | '\\' => decoded.push(character),
                _ => return Err(malformed("unsupported quoted-string escape")),
            }
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character.is_control() || character == '"' {
            return Err(malformed("unescaped quote or control character in value"));
        } else {
            decoded.push(character);
        }
        if decoded.len() > MAX_VALUE_BYTES {
            return Err(malformed("variable value exceeds the byte limit"));
        }
    }
    if escaped {
        return Err(malformed("unterminated quoted-string escape"));
    }
    Ok(decoded)
}

fn check_line(line: &str) -> Result<(), NutProtocolError> {
    if line.is_empty() {
        return Err(malformed("empty response line"));
    }
    if line.len() > MAX_LINE_BYTES {
        return Err(NutProtocolError::LineTooLong);
    }
    if line.chars().any(|character| character.is_control()) {
        return Err(malformed("response line contains a control character"));
    }
    Ok(())
}

fn malformed(message: impl Into<String>) -> NutProtocolError {
    NutProtocolError::Malformed(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_exact_read_only_request() {
        assert_eq!(
            encode_list_var_request("ups-1").unwrap(),
            b"LIST VAR ups-1\n"
        );
        assert!(encode_list_var_request("ups 1").is_err());
    }

    #[test]
    fn decodes_correlated_variables_and_escapes() {
        let response = decode_list_var_response(
            b"BEGIN LIST VAR ups-1\r\nVAR ups-1 battery.charge \"97.5\"\r\nVAR ups-1 device.model \"Rack \\\"A\\\"\\\\B\"\r\nEND LIST VAR ups-1\r\n",
            "ups-1",
        )
        .unwrap();
        assert_eq!(response.variables[0].value, "97.5");
        assert_eq!(response.variables[1].value, "Rack \"A\"\\B");
    }

    #[test]
    fn rejects_server_errors_and_mismatched_lists() {
        assert!(matches!(
            decode_list_var_response(b"ERR UNKNOWN-UPS\n", "ups-1"),
            Err(NutProtocolError::ServerError(_))
        ));
        assert!(matches!(
            decode_list_var_response(b"BEGIN LIST VAR ups-2\nEND LIST VAR ups-2\n", "ups-1"),
            Err(NutProtocolError::UpsMismatch { .. })
        ));
    }

    #[test]
    fn rejects_duplicates_trailing_records_and_missing_end() {
        let duplicate = b"BEGIN LIST VAR ups\nVAR ups x \"1\"\nVAR ups x \"2\"\nEND LIST VAR ups\n";
        assert!(matches!(
            decode_list_var_response(duplicate, "ups"),
            Err(NutProtocolError::DuplicateVariable(_))
        ));
        let trailing = b"BEGIN LIST VAR ups\nEND LIST VAR ups\nVAR ups x \"1\"\n";
        assert!(decode_list_var_response(trailing, "ups").is_err());
        assert!(decode_list_var_response(b"BEGIN LIST VAR ups\n", "ups").is_err());
    }

    #[test]
    fn rejects_invalid_quoting_and_bounds() {
        assert!(decode_list_var_response(
            b"BEGIN LIST VAR ups\nVAR ups x \"bad\\n\"\nEND LIST VAR ups\n",
            "ups"
        )
        .is_err());
        let oversized = vec![b'x'; MAX_RESPONSE_BYTES + 1];
        assert!(matches!(
            decode_list_var_response(&oversized, "ups"),
            Err(NutProtocolError::ResponseTooLarge)
        ));
    }
}
