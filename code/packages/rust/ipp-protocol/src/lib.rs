//! Strict bounded IPP/1.1 printer status framing.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;

pub const VERSION: &str = "0.1.0";
pub const GET_PRINTER_ATTRIBUTES_OPERATION: u16 = 0x000b;
pub const SUCCESSFUL_OK_STATUS: u16 = 0x0000;
pub const MAX_MESSAGE_BYTES: usize = 64 * 1024;
pub const MAX_TEXT_BYTES: usize = 255;
pub const MAX_STATE_REASONS: usize = 32;

const OPERATION_ATTRIBUTES_TAG: u8 = 0x01;
const PRINTER_ATTRIBUTES_TAG: u8 = 0x04;
const END_OF_ATTRIBUTES_TAG: u8 = 0x03;
const INTEGER_TAG: u8 = 0x21;
const BOOLEAN_TAG: u8 = 0x22;
const ENUM_TAG: u8 = 0x23;
const TEXT_WITHOUT_LANGUAGE_TAG: u8 = 0x41;
const NAME_WITHOUT_LANGUAGE_TAG: u8 = 0x42;
const KEYWORD_TAG: u8 = 0x44;
const URI_TAG: u8 = 0x45;
const CHARSET_TAG: u8 = 0x47;
const NATURAL_LANGUAGE_TAG: u8 = 0x48;

pub const REQUESTED_ATTRIBUTES: [&str; 9] = [
    "printer-name",
    "printer-info",
    "printer-location",
    "printer-make-and-model",
    "printer-state",
    "printer-state-reasons",
    "printer-is-accepting-jobs",
    "queued-job-count",
    "printer-up-time",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IppProtocolError {
    Validation(String),
    Truncated,
    MessageTooLarge {
        limit: usize,
    },
    UnsupportedVersion {
        major: u8,
        minor: u8,
    },
    Status(u16),
    RequestIdMismatch {
        expected: u32,
        actual: u32,
    },
    UnexpectedGroup(u8),
    UnexpectedAttribute(String),
    InvalidValue {
        attribute: String,
        expected: &'static str,
    },
    DuplicateAttribute(String),
    MissingAttribute(&'static str),
    TrailingData,
}

impl fmt::Display for IppProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid IPP input: {message}"),
            Self::Truncated => formatter.write_str("truncated IPP message"),
            Self::MessageTooLarge { limit } => {
                write!(formatter, "IPP message exceeds {limit} bytes")
            }
            Self::UnsupportedVersion { major, minor } => {
                write!(formatter, "unsupported IPP version {major}.{minor}")
            }
            Self::Status(status) => {
                write!(formatter, "IPP response returned status 0x{status:04x}")
            }
            Self::RequestIdMismatch { expected, actual } => write!(
                formatter,
                "IPP response request id {actual} does not match {expected}"
            ),
            Self::UnexpectedGroup(tag) => write!(formatter, "unexpected IPP group tag 0x{tag:02x}"),
            Self::UnexpectedAttribute(attribute) => {
                write!(formatter, "unexpected IPP attribute `{attribute}`")
            }
            Self::InvalidValue {
                attribute,
                expected,
            } => {
                write!(formatter, "IPP attribute `{attribute}` must be {expected}")
            }
            Self::DuplicateAttribute(attribute) => {
                write!(formatter, "duplicate IPP attribute `{attribute}`")
            }
            Self::MissingAttribute(attribute) => {
                write!(formatter, "IPP response is missing `{attribute}`")
            }
            Self::TrailingData => formatter.write_str("IPP response contains trailing data"),
        }
    }
}

impl std::error::Error for IppProtocolError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrinterState {
    Idle,
    Processing,
    Stopped,
    Unknown(i32),
}

impl PrinterState {
    pub const fn from_code(code: i32) -> Self {
        match code {
            3 => Self::Idle,
            4 => Self::Processing,
            5 => Self::Stopped,
            other => Self::Unknown(other),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Processing => "processing",
            Self::Stopped => "stopped",
            Self::Unknown(_) => "unknown",
        }
    }

    pub const fn code(self) -> i32 {
        match self {
            Self::Idle => 3,
            Self::Processing => 4,
            Self::Stopped => 5,
            Self::Unknown(code) => code,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrinterAttributes {
    pub printer_name: String,
    pub printer_info: Option<String>,
    pub printer_location: Option<String>,
    pub printer_make_and_model: String,
    pub printer_state: PrinterState,
    pub printer_state_reasons: Vec<String>,
    pub printer_is_accepting_jobs: bool,
    pub queued_job_count: u32,
    pub printer_up_time_seconds: u32,
}

pub fn encode_get_printer_attributes(
    request_id: u32,
    printer_uri: &str,
) -> Result<Vec<u8>, IppProtocolError> {
    if request_id == 0 {
        return Err(IppProtocolError::Validation(
            "request id must be non-zero".to_string(),
        ));
    }
    validate_uri(printer_uri)?;
    let mut bytes = Vec::with_capacity(512);
    bytes.extend_from_slice(&[1, 1]);
    bytes.extend_from_slice(&GET_PRINTER_ATTRIBUTES_OPERATION.to_be_bytes());
    bytes.extend_from_slice(&request_id.to_be_bytes());
    bytes.push(OPERATION_ATTRIBUTES_TAG);
    push_attribute(&mut bytes, CHARSET_TAG, "attributes-charset", b"utf-8")?;
    push_attribute(
        &mut bytes,
        NATURAL_LANGUAGE_TAG,
        "attributes-natural-language",
        b"en-us",
    )?;
    push_attribute(&mut bytes, URI_TAG, "printer-uri", printer_uri.as_bytes())?;
    for (index, attribute) in REQUESTED_ATTRIBUTES.iter().enumerate() {
        push_attribute(
            &mut bytes,
            KEYWORD_TAG,
            if index == 0 {
                "requested-attributes"
            } else {
                ""
            },
            attribute.as_bytes(),
        )?;
    }
    bytes.push(END_OF_ATTRIBUTES_TAG);
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err(IppProtocolError::MessageTooLarge {
            limit: MAX_MESSAGE_BYTES,
        });
    }
    Ok(bytes)
}

pub fn decode_get_printer_attributes_response(
    bytes: &[u8],
    expected_request_id: u32,
) -> Result<PrinterAttributes, IppProtocolError> {
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err(IppProtocolError::MessageTooLarge {
            limit: MAX_MESSAGE_BYTES,
        });
    }
    let mut cursor = Cursor::new(bytes);
    let major = cursor.byte()?;
    let minor = cursor.byte()?;
    if (major, minor) != (1, 1) {
        return Err(IppProtocolError::UnsupportedVersion { major, minor });
    }
    let status = cursor.u16()?;
    if status != SUCCESSFUL_OK_STATUS {
        return Err(IppProtocolError::Status(status));
    }
    let actual_request_id = cursor.u32()?;
    if actual_request_id != expected_request_id {
        return Err(IppProtocolError::RequestIdMismatch {
            expected: expected_request_id,
            actual: actual_request_id,
        });
    }

    let mut operation = BTreeMap::<String, Vec<TaggedValue>>::new();
    let mut printer = BTreeMap::<String, Vec<TaggedValue>>::new();
    let mut group = None;
    let mut seen_operation = false;
    let mut seen_printer = false;
    let mut previous_name = None::<String>;
    let mut ended = false;
    while !cursor.is_empty() {
        let tag = cursor.byte()?;
        match tag {
            OPERATION_ATTRIBUTES_TAG | PRINTER_ATTRIBUTES_TAG => {
                let valid = match tag {
                    OPERATION_ATTRIBUTES_TAG => !seen_operation && !seen_printer,
                    PRINTER_ATTRIBUTES_TAG => seen_operation && !seen_printer,
                    _ => false,
                };
                if ended || !valid {
                    return Err(IppProtocolError::UnexpectedGroup(tag));
                }
                seen_operation |= tag == OPERATION_ATTRIBUTES_TAG;
                seen_printer |= tag == PRINTER_ATTRIBUTES_TAG;
                group = Some(tag);
                previous_name = None;
            }
            END_OF_ATTRIBUTES_TAG => {
                ended = true;
                if !cursor.is_empty() {
                    return Err(IppProtocolError::TrailingData);
                }
                break;
            }
            value_tag if value_tag >= 0x10 => {
                let current_group = group.ok_or(IppProtocolError::UnexpectedGroup(value_tag))?;
                if ended {
                    return Err(IppProtocolError::TrailingData);
                }
                let name_length = usize::from(cursor.u16()?);
                let name = if name_length == 0 {
                    previous_name
                        .clone()
                        .ok_or_else(|| IppProtocolError::InvalidValue {
                            attribute: "<additional-value>".to_string(),
                            expected: "preceded by a named attribute",
                        })?
                } else {
                    let name = parse_name(cursor.take(name_length)?)?;
                    previous_name = Some(name.clone());
                    name
                };
                let value_length = usize::from(cursor.u16()?);
                let value = cursor.take(value_length)?.to_vec();
                let target = match current_group {
                    OPERATION_ATTRIBUTES_TAG => &mut operation,
                    PRINTER_ATTRIBUTES_TAG => &mut printer,
                    other => return Err(IppProtocolError::UnexpectedGroup(other)),
                };
                if name_length != 0 && target.contains_key(&name) {
                    return Err(IppProtocolError::DuplicateAttribute(name));
                }
                target.entry(name).or_default().push(TaggedValue {
                    tag: value_tag,
                    bytes: value,
                });
            }
            other => return Err(IppProtocolError::UnexpectedGroup(other)),
        }
    }
    if !ended {
        return Err(IppProtocolError::Truncated);
    }
    validate_operation_attributes(&operation)?;
    validate_known_printer_attributes(&printer)?;

    Ok(PrinterAttributes {
        printer_name: required_text(&printer, "printer-name", NAME_WITHOUT_LANGUAGE_TAG)?,
        printer_info: optional_text(&printer, "printer-info", TEXT_WITHOUT_LANGUAGE_TAG)?,
        printer_location: optional_text(&printer, "printer-location", TEXT_WITHOUT_LANGUAGE_TAG)?,
        printer_make_and_model: required_text(
            &printer,
            "printer-make-and-model",
            TEXT_WITHOUT_LANGUAGE_TAG,
        )?,
        printer_state: required_printer_state(&printer)?,
        printer_state_reasons: required_keywords(&printer, "printer-state-reasons")?,
        printer_is_accepting_jobs: required_boolean(&printer, "printer-is-accepting-jobs")?,
        queued_job_count: required_nonnegative_integer(&printer, "queued-job-count")?,
        printer_up_time_seconds: required_nonnegative_integer(&printer, "printer-up-time")?,
    })
}

#[derive(Debug, Clone)]
struct TaggedValue {
    tag: u8,
    bytes: Vec<u8>,
}

fn push_attribute(
    bytes: &mut Vec<u8>,
    tag: u8,
    name: &str,
    value: &[u8],
) -> Result<(), IppProtocolError> {
    let name_length = u16::try_from(name.len()).map_err(|_| {
        IppProtocolError::Validation("attribute name exceeds IPP length".to_string())
    })?;
    let value_length = u16::try_from(value.len()).map_err(|_| {
        IppProtocolError::Validation("attribute value exceeds IPP length".to_string())
    })?;
    bytes.push(tag);
    bytes.extend_from_slice(&name_length.to_be_bytes());
    bytes.extend_from_slice(name.as_bytes());
    bytes.extend_from_slice(&value_length.to_be_bytes());
    bytes.extend_from_slice(value);
    Ok(())
}

fn validate_uri(uri: &str) -> Result<(), IppProtocolError> {
    if uri.len() < 7
        || uri.len() > 1024
        || !uri.starts_with("ipp://")
        || !uri.is_ascii()
        || uri.bytes().any(|byte| byte <= 0x20 || byte == 0x7f)
    {
        return Err(IppProtocolError::Validation(
            "printer URI must be a bounded printable ipp URI".to_string(),
        ));
    }
    Ok(())
}

fn parse_name(bytes: &[u8]) -> Result<String, IppProtocolError> {
    if bytes.is_empty()
        || bytes.len() > MAX_TEXT_BYTES
        || !bytes.iter().all(u8::is_ascii)
        || bytes.iter().any(|byte| *byte <= 0x20 || *byte == 0x7f)
    {
        return Err(IppProtocolError::InvalidValue {
            attribute: "<attribute-name>".to_string(),
            expected: "bounded printable ASCII",
        });
    }
    String::from_utf8(bytes.to_vec()).map_err(|_| IppProtocolError::InvalidValue {
        attribute: "<attribute-name>".to_string(),
        expected: "ASCII",
    })
}

fn validate_operation_attributes(
    attributes: &BTreeMap<String, Vec<TaggedValue>>,
) -> Result<(), IppProtocolError> {
    for name in attributes.keys() {
        if !matches!(
            name.as_str(),
            "attributes-charset"
                | "attributes-natural-language"
                | "status-message"
                | "detailed-status-message"
        ) {
            return Err(IppProtocolError::UnexpectedAttribute(name.clone()));
        }
    }
    let charset = required_text(attributes, "attributes-charset", CHARSET_TAG)?;
    if !charset.eq_ignore_ascii_case("utf-8") {
        return Err(IppProtocolError::InvalidValue {
            attribute: "attributes-charset".to_string(),
            expected: "utf-8",
        });
    }
    let language = required_text(
        attributes,
        "attributes-natural-language",
        NATURAL_LANGUAGE_TAG,
    )?;
    if language.len() > 35
        || !language.is_ascii()
        || language
            .bytes()
            .any(|byte| !(byte.is_ascii_alphanumeric() || byte == b'-'))
    {
        return Err(IppProtocolError::InvalidValue {
            attribute: "attributes-natural-language".to_string(),
            expected: "a bounded language tag",
        });
    }
    for name in ["status-message", "detailed-status-message"] {
        if attributes.contains_key(name) {
            let _ = optional_text(attributes, name, TEXT_WITHOUT_LANGUAGE_TAG)?;
        }
    }
    Ok(())
}

fn validate_known_printer_attributes(
    attributes: &BTreeMap<String, Vec<TaggedValue>>,
) -> Result<(), IppProtocolError> {
    for name in attributes.keys() {
        if !REQUESTED_ATTRIBUTES.contains(&name.as_str()) {
            return Err(IppProtocolError::UnexpectedAttribute(name.clone()));
        }
    }
    Ok(())
}

fn required_text(
    attributes: &BTreeMap<String, Vec<TaggedValue>>,
    name: &'static str,
    expected_tag: u8,
) -> Result<String, IppProtocolError> {
    optional_text(attributes, name, expected_tag)?.ok_or(IppProtocolError::MissingAttribute(name))
}

fn optional_text(
    attributes: &BTreeMap<String, Vec<TaggedValue>>,
    name: &str,
    expected_tag: u8,
) -> Result<Option<String>, IppProtocolError> {
    let Some(values) = attributes.get(name) else {
        return Ok(None);
    };
    let value = single(values, name)?;
    if value.tag != expected_tag {
        return Err(IppProtocolError::InvalidValue {
            attribute: name.to_string(),
            expected: "the standardized string syntax",
        });
    }
    if value.bytes.is_empty()
        || value.bytes.len() > MAX_TEXT_BYTES
        || value.bytes.iter().any(|byte| byte.is_ascii_control())
    {
        return Err(IppProtocolError::InvalidValue {
            attribute: name.to_string(),
            expected: "non-empty bounded UTF-8 text without control characters",
        });
    }
    String::from_utf8(value.bytes.clone())
        .map(Some)
        .map_err(|_| IppProtocolError::InvalidValue {
            attribute: name.to_string(),
            expected: "UTF-8 text",
        })
}

fn required_i32(
    attributes: &BTreeMap<String, Vec<TaggedValue>>,
    name: &'static str,
    expected_tag: u8,
) -> Result<i32, IppProtocolError> {
    let values = attributes
        .get(name)
        .ok_or(IppProtocolError::MissingAttribute(name))?;
    let value = single(values, name)?;
    if value.tag != expected_tag || value.bytes.len() != 4 {
        return Err(IppProtocolError::InvalidValue {
            attribute: name.to_string(),
            expected: "one four-byte integer",
        });
    }
    Ok(i32::from_be_bytes(
        value
            .bytes
            .as_slice()
            .try_into()
            .map_err(|_| IppProtocolError::Truncated)?,
    ))
}

fn required_nonnegative_integer(
    attributes: &BTreeMap<String, Vec<TaggedValue>>,
    name: &'static str,
) -> Result<u32, IppProtocolError> {
    let value = required_i32(attributes, name, INTEGER_TAG)?;
    u32::try_from(value).map_err(|_| IppProtocolError::InvalidValue {
        attribute: name.to_string(),
        expected: "a non-negative integer",
    })
}

fn required_printer_state(
    attributes: &BTreeMap<String, Vec<TaggedValue>>,
) -> Result<PrinterState, IppProtocolError> {
    let code = required_i32(attributes, "printer-state", ENUM_TAG)?;
    if code < 3 {
        return Err(IppProtocolError::InvalidValue {
            attribute: "printer-state".to_string(),
            expected: "a standardized or future positive printer-state enum",
        });
    }
    Ok(PrinterState::from_code(code))
}

fn required_boolean(
    attributes: &BTreeMap<String, Vec<TaggedValue>>,
    name: &'static str,
) -> Result<bool, IppProtocolError> {
    let values = attributes
        .get(name)
        .ok_or(IppProtocolError::MissingAttribute(name))?;
    let value = single(values, name)?;
    if value.tag != BOOLEAN_TAG || value.bytes.len() != 1 || value.bytes[0] > 1 {
        return Err(IppProtocolError::InvalidValue {
            attribute: name.to_string(),
            expected: "one canonical boolean octet",
        });
    }
    Ok(value.bytes[0] == 1)
}

fn required_keywords(
    attributes: &BTreeMap<String, Vec<TaggedValue>>,
    name: &'static str,
) -> Result<Vec<String>, IppProtocolError> {
    let values = attributes
        .get(name)
        .ok_or(IppProtocolError::MissingAttribute(name))?;
    if values.is_empty() || values.len() > MAX_STATE_REASONS {
        return Err(IppProtocolError::InvalidValue {
            attribute: name.to_string(),
            expected: "1..=32 keyword values",
        });
    }
    let mut output = Vec::with_capacity(values.len());
    for value in values {
        if value.tag != KEYWORD_TAG
            || value.bytes.is_empty()
            || value.bytes.len() > MAX_TEXT_BYTES
            || !value.bytes.iter().all(u8::is_ascii)
            || value
                .bytes
                .iter()
                .any(|byte| byte.is_ascii_control() || *byte == b' ')
        {
            return Err(IppProtocolError::InvalidValue {
                attribute: name.to_string(),
                expected: "bounded printable ASCII keywords",
            });
        }
        output.push(String::from_utf8(value.bytes.clone()).map_err(|_| {
            IppProtocolError::InvalidValue {
                attribute: name.to_string(),
                expected: "ASCII keywords",
            }
        })?);
    }
    output.sort();
    output.dedup();
    if output.len() != values.len() {
        return Err(IppProtocolError::DuplicateAttribute(name.to_string()));
    }
    Ok(output)
}

fn single<'a>(values: &'a [TaggedValue], name: &str) -> Result<&'a TaggedValue, IppProtocolError> {
    if values.len() != 1 {
        return Err(IppProtocolError::DuplicateAttribute(name.to_string()));
    }
    Ok(&values[0])
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn byte(&mut self) -> Result<u8, IppProtocolError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, IppProtocolError> {
        Ok(u16::from_be_bytes(
            self.take(2)?
                .try_into()
                .map_err(|_| IppProtocolError::Truncated)?,
        ))
    }

    fn u32(&mut self) -> Result<u32, IppProtocolError> {
        Ok(u32::from_be_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| IppProtocolError::Truncated)?,
        ))
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], IppProtocolError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(IppProtocolError::Truncated)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(IppProtocolError::Truncated)?;
        self.offset = end;
        Ok(value)
    }

    fn is_empty(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(request_id: u32) -> Vec<u8> {
        let mut bytes = vec![1, 1, 0, 0];
        bytes.extend_from_slice(&request_id.to_be_bytes());
        bytes.push(OPERATION_ATTRIBUTES_TAG);
        push_attribute(&mut bytes, CHARSET_TAG, "attributes-charset", b"utf-8").unwrap();
        push_attribute(
            &mut bytes,
            NATURAL_LANGUAGE_TAG,
            "attributes-natural-language",
            b"en-us",
        )
        .unwrap();
        bytes.push(PRINTER_ATTRIBUTES_TAG);
        push_attribute(
            &mut bytes,
            NAME_WITHOUT_LANGUAGE_TAG,
            "printer-name",
            b"Office",
        )
        .unwrap();
        push_attribute(
            &mut bytes,
            TEXT_WITHOUT_LANGUAGE_TAG,
            "printer-info",
            b"Main printer",
        )
        .unwrap();
        push_attribute(
            &mut bytes,
            TEXT_WITHOUT_LANGUAGE_TAG,
            "printer-location",
            b"Second floor",
        )
        .unwrap();
        push_attribute(
            &mut bytes,
            TEXT_WITHOUT_LANGUAGE_TAG,
            "printer-make-and-model",
            b"Example Laser 2",
        )
        .unwrap();
        push_attribute(&mut bytes, ENUM_TAG, "printer-state", &3i32.to_be_bytes()).unwrap();
        push_attribute(&mut bytes, KEYWORD_TAG, "printer-state-reasons", b"none").unwrap();
        push_attribute(&mut bytes, BOOLEAN_TAG, "printer-is-accepting-jobs", &[1]).unwrap();
        push_attribute(
            &mut bytes,
            INTEGER_TAG,
            "queued-job-count",
            &2i32.to_be_bytes(),
        )
        .unwrap();
        push_attribute(
            &mut bytes,
            INTEGER_TAG,
            "printer-up-time",
            &90i32.to_be_bytes(),
        )
        .unwrap();
        bytes.push(END_OF_ATTRIBUTES_TAG);
        bytes
    }

    #[test]
    fn encodes_fixed_get_printer_attributes_request() {
        let bytes = encode_get_printer_attributes(7, "ipp://127.0.0.1:631/ipp/print").unwrap();
        assert_eq!(&bytes[..8], &[1, 1, 0, 11, 0, 0, 0, 7]);
        assert_eq!(bytes.last(), Some(&END_OF_ATTRIBUTES_TAG));
        for name in REQUESTED_ATTRIBUTES {
            assert!(bytes
                .windows(name.len())
                .any(|window| window == name.as_bytes()));
        }
        assert!(!bytes.windows(9).any(|window| window == b"Print-Job"));
    }

    #[test]
    fn decodes_correlated_printer_status() {
        let decoded = decode_get_printer_attributes_response(&response(9), 9).unwrap();
        assert_eq!(decoded.printer_name, "Office");
        assert_eq!(decoded.printer_make_and_model, "Example Laser 2");
        assert_eq!(decoded.printer_state, PrinterState::Idle);
        assert_eq!(decoded.printer_state_reasons, vec!["none"]);
        assert!(decoded.printer_is_accepting_jobs);
        assert_eq!(decoded.queued_job_count, 2);
        assert_eq!(decoded.printer_up_time_seconds, 90);
    }

    #[test]
    fn preserves_future_printer_state() {
        let mut bytes = response(4);
        let position = bytes
            .windows("printer-state".len())
            .position(|window| window == b"printer-state")
            .unwrap();
        let value_offset = position + "printer-state".len() + 2;
        bytes[value_offset..value_offset + 4].copy_from_slice(&99i32.to_be_bytes());
        let decoded = decode_get_printer_attributes_response(&bytes, 4).unwrap();
        assert_eq!(decoded.printer_state, PrinterState::Unknown(99));
    }

    #[test]
    fn rejects_mismatched_request_and_non_success_status() {
        assert!(matches!(
            decode_get_printer_attributes_response(&response(1), 2),
            Err(IppProtocolError::RequestIdMismatch { .. })
        ));
        let mut bytes = response(1);
        bytes[2..4].copy_from_slice(&0x0401u16.to_be_bytes());
        assert_eq!(
            decode_get_printer_attributes_response(&bytes, 1),
            Err(IppProtocolError::Status(0x0401))
        );
    }

    #[test]
    fn rejects_wrong_type_duplicate_unknown_and_trailing_data() {
        let mut wrong_type = response(1);
        let position = wrong_type
            .windows("queued-job-count".len())
            .position(|window| window == b"queued-job-count")
            .unwrap();
        wrong_type[position - 3] = KEYWORD_TAG;
        assert!(matches!(
            decode_get_printer_attributes_response(&wrong_type, 1),
            Err(IppProtocolError::InvalidValue { .. })
        ));

        let mut duplicate = response(1);
        let end = duplicate.pop().unwrap();
        push_attribute(
            &mut duplicate,
            NAME_WITHOUT_LANGUAGE_TAG,
            "printer-name",
            b"Other",
        )
        .unwrap();
        duplicate.push(end);
        assert!(matches!(
            decode_get_printer_attributes_response(&duplicate, 1),
            Err(IppProtocolError::DuplicateAttribute(_))
        ));

        let mut unknown = response(1);
        let end = unknown.pop().unwrap();
        push_attribute(&mut unknown, KEYWORD_TAG, "job-state", b"pending").unwrap();
        unknown.push(end);
        assert!(matches!(
            decode_get_printer_attributes_response(&unknown, 1),
            Err(IppProtocolError::UnexpectedAttribute(_))
        ));

        let mut trailing = response(1);
        trailing.push(0);
        assert_eq!(
            decode_get_printer_attributes_response(&trailing, 1),
            Err(IppProtocolError::TrailingData)
        );
    }

    #[test]
    fn rejects_missing_and_malformed_values() {
        let mut truncated = response(1);
        truncated.pop();
        assert_eq!(
            decode_get_printer_attributes_response(&truncated, 1),
            Err(IppProtocolError::Truncated)
        );

        let mut negative = response(1);
        let position = negative
            .windows("queued-job-count".len())
            .position(|window| window == b"queued-job-count")
            .unwrap();
        let value_offset = position + "queued-job-count".len() + 2;
        negative[value_offset..value_offset + 4].copy_from_slice(&(-1i32).to_be_bytes());
        assert!(matches!(
            decode_get_printer_attributes_response(&negative, 1),
            Err(IppProtocolError::InvalidValue { .. })
        ));

        let mut invalid_state = response(1);
        let position = invalid_state
            .windows("printer-state".len())
            .position(|window| window == b"printer-state")
            .unwrap();
        let value_offset = position + "printer-state".len() + 2;
        invalid_state[value_offset..value_offset + 4].copy_from_slice(&2i32.to_be_bytes());
        assert!(matches!(
            decode_get_printer_attributes_response(&invalid_state, 1),
            Err(IppProtocolError::InvalidValue { .. })
        ));
    }

    #[test]
    fn rejects_missing_repeated_or_out_of_order_groups() {
        let mut printer_first = response(1);
        printer_first[8] = PRINTER_ATTRIBUTES_TAG;
        assert_eq!(
            decode_get_printer_attributes_response(&printer_first, 1),
            Err(IppProtocolError::UnexpectedGroup(PRINTER_ATTRIBUTES_TAG))
        );

        let mut repeated_operation = response(1);
        let printer_group = repeated_operation
            .iter()
            .position(|byte| *byte == PRINTER_ATTRIBUTES_TAG)
            .unwrap();
        repeated_operation[printer_group] = OPERATION_ATTRIBUTES_TAG;
        assert_eq!(
            decode_get_printer_attributes_response(&repeated_operation, 1),
            Err(IppProtocolError::UnexpectedGroup(OPERATION_ATTRIBUTES_TAG))
        );

        let mut missing_printer = response(1);
        missing_printer.truncate(printer_group);
        missing_printer.push(END_OF_ATTRIBUTES_TAG);
        assert!(matches!(
            decode_get_printer_attributes_response(&missing_printer, 1),
            Err(IppProtocolError::MissingAttribute(_))
        ));
    }

    #[test]
    fn rejects_unbounded_or_unsafe_requests() {
        assert!(encode_get_printer_attributes(0, "ipp://127.0.0.1/ipp/print").is_err());
        assert!(encode_get_printer_attributes(1, "http://127.0.0.1/ipp/print").is_err());
        assert!(encode_get_printer_attributes(1, "ipp://127.0.0.1/ipp/\r\nprint").is_err());
    }
}
