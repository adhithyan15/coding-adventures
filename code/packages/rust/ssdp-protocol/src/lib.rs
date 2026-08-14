//! Strict bounded SSDP M-SEARCH request and response framing.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;

pub const VERSION: &str = "0.1.0";
pub const SSDP_IPV4_HOST: &str = "239.255.255.250:1900";
pub const MAX_DATAGRAM_BYTES: usize = 8 * 1024;
pub const MAX_HEADER_LINES: usize = 64;
pub const MAX_HEADER_LINE_BYTES: usize = 1024;
pub const MAX_SEARCH_TARGET_BYTES: usize = 256;
pub const MAX_USN_BYTES: usize = 512;
pub const MAX_CACHE_AGE_SECONDS: u32 = 7 * 24 * 60 * 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchRequest {
    pub search_target: String,
    pub maximum_wait_seconds: u8,
    pub user_agent: String,
}

impl Default for SearchRequest {
    fn default() -> Self {
        Self {
            search_target: "ssdp:all".to_string(),
            maximum_wait_seconds: 2,
            user_agent: "coding-adventures/0.1 UPnP/2.0 smart-home-ssdp/0.1".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResponse {
    pub location: String,
    pub server: String,
    pub search_target: String,
    pub unique_service_name: String,
    pub unique_device_name: String,
    pub usn_target: Option<String>,
    pub max_age_seconds: u32,
    pub boot_id: Option<u32>,
    pub config_id: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SsdpError {
    InvalidRequestField(&'static str),
    InvalidSearchTarget,
    InvalidMaximumWait(u8),
    DatagramTooLarge {
        actual: usize,
        maximum: usize,
    },
    InvalidEncoding,
    MissingTerminator,
    UnexpectedBody,
    InvalidStatusLine,
    TooManyHeaders,
    HeaderLineTooLong,
    InvalidHeaderLine,
    DuplicateHeader(String),
    MissingHeader(&'static str),
    InvalidExt,
    InvalidCacheControl,
    InvalidUnsignedHeader(&'static str),
    InvalidUsn,
    SearchTargetMismatch {
        expected: String,
        actual: String,
    },
    UsnTargetMismatch {
        expected: String,
        actual: Option<String>,
    },
}

impl fmt::Display for SsdpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequestField(field) => write!(formatter, "invalid SSDP {field}"),
            Self::InvalidSearchTarget => formatter.write_str("invalid SSDP search target"),
            Self::InvalidMaximumWait(value) => {
                write!(formatter, "SSDP MX must be between 1 and 5, got {value}")
            }
            Self::DatagramTooLarge { actual, maximum } => write!(
                formatter,
                "SSDP datagram is {actual} bytes, exceeding the {maximum}-byte limit"
            ),
            Self::InvalidEncoding => formatter.write_str("SSDP datagram must be ASCII"),
            Self::MissingTerminator => {
                formatter.write_str("SSDP datagram must end with one empty header line")
            }
            Self::UnexpectedBody => {
                formatter.write_str("SSDP search response must not have a body")
            }
            Self::InvalidStatusLine => {
                formatter.write_str("SSDP search response must be HTTP/1.1 200 OK")
            }
            Self::TooManyHeaders => write!(
                formatter,
                "SSDP response exceeds the {MAX_HEADER_LINES}-header limit"
            ),
            Self::HeaderLineTooLong => write!(
                formatter,
                "SSDP header exceeds the {MAX_HEADER_LINE_BYTES}-byte limit"
            ),
            Self::InvalidHeaderLine => formatter.write_str("invalid SSDP header line"),
            Self::DuplicateHeader(name) => write!(formatter, "duplicate SSDP header `{name}`"),
            Self::MissingHeader(name) => write!(formatter, "SSDP response is missing {name}"),
            Self::InvalidExt => formatter.write_str("SSDP EXT header must be empty"),
            Self::InvalidCacheControl => formatter.write_str("invalid SSDP CACHE-CONTROL max-age"),
            Self::InvalidUnsignedHeader(name) => {
                write!(formatter, "invalid SSDP {name} unsigned integer")
            }
            Self::InvalidUsn => formatter.write_str("invalid SSDP USN/UDN"),
            Self::SearchTargetMismatch { expected, actual } => write!(
                formatter,
                "SSDP response target `{actual}` does not match requested `{expected}`"
            ),
            Self::UsnTargetMismatch { expected, actual } => write!(
                formatter,
                "SSDP USN target {:?} does not match response target `{expected}`",
                actual
            ),
        }
    }
}

impl std::error::Error for SsdpError {}

pub fn encode_m_search(request: &SearchRequest) -> Result<Vec<u8>, SsdpError> {
    validate_search_target(&request.search_target)?;
    if !(1..=5).contains(&request.maximum_wait_seconds) {
        return Err(SsdpError::InvalidMaximumWait(request.maximum_wait_seconds));
    }
    validate_header_value("USER-AGENT", &request.user_agent, 256)?;
    Ok(format!(
        "M-SEARCH * HTTP/1.1\r\nHOST: {SSDP_IPV4_HOST}\r\nMAN: \"ssdp:discover\"\r\nMX: {}\r\nST: {}\r\nUSER-AGENT: {}\r\n\r\n",
        request.maximum_wait_seconds, request.search_target, request.user_agent
    )
    .into_bytes())
}

pub fn decode_search_response(
    bytes: &[u8],
    requested_target: &str,
) -> Result<SearchResponse, SsdpError> {
    validate_search_target(requested_target)?;
    if bytes.len() > MAX_DATAGRAM_BYTES {
        return Err(SsdpError::DatagramTooLarge {
            actual: bytes.len(),
            maximum: MAX_DATAGRAM_BYTES,
        });
    }
    if !bytes.is_ascii() {
        return Err(SsdpError::InvalidEncoding);
    }
    let source = std::str::from_utf8(bytes).map_err(|_| SsdpError::InvalidEncoding)?;
    let Some(head) = source.strip_suffix("\r\n\r\n") else {
        return Err(SsdpError::MissingTerminator);
    };
    if head.contains("\r\n\r\n") {
        return Err(SsdpError::UnexpectedBody);
    }

    let mut lines = head.split("\r\n");
    if lines.next() != Some("HTTP/1.1 200 OK") {
        return Err(SsdpError::InvalidStatusLine);
    }
    let mut headers = BTreeMap::<String, String>::new();
    for (index, line) in lines.enumerate() {
        if index >= MAX_HEADER_LINES {
            return Err(SsdpError::TooManyHeaders);
        }
        if line.len() > MAX_HEADER_LINE_BYTES {
            return Err(SsdpError::HeaderLineTooLong);
        }
        if line.starts_with(' ') || line.starts_with('\t') {
            return Err(SsdpError::InvalidHeaderLine);
        }
        let (name, value) = line.split_once(':').ok_or(SsdpError::InvalidHeaderLine)?;
        if name.is_empty() || !name.bytes().all(is_header_name_byte) {
            return Err(SsdpError::InvalidHeaderLine);
        }
        let name = name.to_ascii_lowercase();
        let value = value.trim().to_string();
        if value.contains('\r') || value.contains('\n') {
            return Err(SsdpError::InvalidHeaderLine);
        }
        if headers.insert(name.clone(), value).is_some() {
            return Err(SsdpError::DuplicateHeader(name));
        }
    }

    let ext = required(&headers, "ext")?;
    if !ext.is_empty() {
        return Err(SsdpError::InvalidExt);
    }
    let location = required_nonempty(&headers, "location")?.to_string();
    let server = required_nonempty(&headers, "server")?.to_string();
    let search_target = required_nonempty(&headers, "st")?.to_string();
    validate_search_target(&search_target)?;
    correlate_search_target(requested_target, &search_target)?;
    let unique_service_name = required_nonempty(&headers, "usn")?.to_string();
    if unique_service_name.len() > MAX_USN_BYTES {
        return Err(SsdpError::InvalidUsn);
    }
    let (unique_device_name, usn_target) = parse_usn(&unique_service_name)?;
    correlate_usn_target(&search_target, &unique_device_name, usn_target.as_deref())?;
    let max_age_seconds = parse_max_age(required_nonempty(&headers, "cache-control")?)?;
    let boot_id = optional_u32(&headers, "bootid.upnp.org")?;
    let config_id = optional_u32(&headers, "configid.upnp.org")?;

    validate_header_value("LOCATION", &location, 2048)?;
    validate_header_value("SERVER", &server, 512)?;
    Ok(SearchResponse {
        location,
        server,
        search_target,
        unique_service_name,
        unique_device_name,
        usn_target,
        max_age_seconds,
        boot_id,
        config_id,
    })
}

fn is_header_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte)
}

fn validate_search_target(value: &str) -> Result<(), SsdpError> {
    if value.is_empty()
        || value.len() > MAX_SEARCH_TARGET_BYTES
        || !value.is_ascii()
        || value.bytes().any(|byte| byte <= b' ' || byte == 0x7f)
    {
        return Err(SsdpError::InvalidSearchTarget);
    }
    let supported = value == "ssdp:all"
        || value == "upnp:rootdevice"
        || value.starts_with("uuid:")
        || value.starts_with("urn:");
    if supported {
        Ok(())
    } else {
        Err(SsdpError::InvalidSearchTarget)
    }
}

fn validate_header_value(
    field: &'static str,
    value: &str,
    maximum: usize,
) -> Result<(), SsdpError> {
    if value.is_empty()
        || value.len() > maximum
        || !value.is_ascii()
        || value.bytes().any(|byte| byte < b' ' || byte == 0x7f)
    {
        Err(SsdpError::InvalidRequestField(field))
    } else {
        Ok(())
    }
}

fn required<'a>(
    headers: &'a BTreeMap<String, String>,
    name: &'static str,
) -> Result<&'a str, SsdpError> {
    headers
        .get(name)
        .map(String::as_str)
        .ok_or(SsdpError::MissingHeader(name))
}

fn required_nonempty<'a>(
    headers: &'a BTreeMap<String, String>,
    name: &'static str,
) -> Result<&'a str, SsdpError> {
    required(headers, name).and_then(|value| {
        if value.is_empty() {
            Err(SsdpError::MissingHeader(name))
        } else {
            Ok(value)
        }
    })
}

fn parse_max_age(value: &str) -> Result<u32, SsdpError> {
    let mut found = None;
    for directive in value.split(',') {
        let directive = directive.trim();
        if let Some(raw) = directive.strip_prefix("max-age=") {
            if found.is_some() || raw.is_empty() || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(SsdpError::InvalidCacheControl);
            }
            found = Some(
                raw.parse::<u32>()
                    .map_err(|_| SsdpError::InvalidCacheControl)?,
            );
        }
    }
    match found {
        Some(value) if (1..=MAX_CACHE_AGE_SECONDS).contains(&value) => Ok(value),
        _ => Err(SsdpError::InvalidCacheControl),
    }
}

fn optional_u32(
    headers: &BTreeMap<String, String>,
    name: &'static str,
) -> Result<Option<u32>, SsdpError> {
    headers
        .get(name)
        .map(|value| {
            if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(SsdpError::InvalidUnsignedHeader(name));
            }
            value
                .parse::<u32>()
                .map_err(|_| SsdpError::InvalidUnsignedHeader(name))
        })
        .transpose()
}

fn parse_usn(value: &str) -> Result<(String, Option<String>), SsdpError> {
    let (udn, target) = match value.split_once("::") {
        Some((udn, target)) if !target.is_empty() && !target.contains("::") => {
            (udn, Some(target.to_string()))
        }
        Some(_) => return Err(SsdpError::InvalidUsn),
        None => (value, None),
    };
    let Some(identifier) = udn.strip_prefix("uuid:") else {
        return Err(SsdpError::InvalidUsn);
    };
    if identifier.is_empty()
        || identifier.len() > 128
        || !identifier.is_ascii()
        || !identifier
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-._:".contains(&byte))
    {
        return Err(SsdpError::InvalidUsn);
    }
    Ok((udn.to_ascii_lowercase(), target))
}

fn correlate_search_target(requested: &str, actual: &str) -> Result<(), SsdpError> {
    if requested == "ssdp:all" || requested == actual {
        Ok(())
    } else {
        Err(SsdpError::SearchTargetMismatch {
            expected: requested.to_string(),
            actual: actual.to_string(),
        })
    }
}

fn correlate_usn_target(
    search_target: &str,
    unique_device_name: &str,
    actual: Option<&str>,
) -> Result<(), SsdpError> {
    if search_target.starts_with("uuid:") {
        if actual.is_none() && unique_device_name.eq_ignore_ascii_case(search_target) {
            return Ok(());
        }
    } else if actual == Some(search_target) {
        return Ok(());
    }
    Err(SsdpError::UsnTargetMismatch {
        expected: search_target.to_string(),
        actual: actual.map(str::to_string),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(extra: &str) -> Vec<u8> {
        format!(
            "HTTP/1.1 200 OK\r\nCACHE-CONTROL: max-age=1800\r\nEXT:\r\nLOCATION: http://192.168.1.10:1400/xml/device.xml\r\nSERVER: TestOS/1.0 UPnP/2.0 Test/1.0\r\nST: upnp:rootdevice\r\nUSN: uuid:device-123::upnp:rootdevice\r\n{extra}\r\n"
        )
        .into_bytes()
    }

    #[test]
    fn encodes_canonical_m_search_request() {
        assert_eq!(
            String::from_utf8(encode_m_search(&SearchRequest::default()).unwrap()).unwrap(),
            "M-SEARCH * HTTP/1.1\r\nHOST: 239.255.255.250:1900\r\nMAN: \"ssdp:discover\"\r\nMX: 2\r\nST: ssdp:all\r\nUSER-AGENT: coding-adventures/0.1 UPnP/2.0 smart-home-ssdp/0.1\r\n\r\n"
        );
    }

    #[test]
    fn request_rejects_header_injection_and_unbounded_wait() {
        let request = SearchRequest {
            search_target: "ssdp:all\r\nX: injected".to_string(),
            ..SearchRequest::default()
        };
        assert_eq!(
            encode_m_search(&request),
            Err(SsdpError::InvalidSearchTarget)
        );
        let request = SearchRequest {
            maximum_wait_seconds: 0,
            ..SearchRequest::default()
        };
        assert_eq!(
            encode_m_search(&request),
            Err(SsdpError::InvalidMaximumWait(0))
        );
    }

    #[test]
    fn decodes_required_response_fields_and_optional_ids() {
        let decoded = decode_search_response(
            &response("BOOTID.UPNP.ORG: 7\r\nCONFIGID.UPNP.ORG: 11\r\n"),
            "upnp:rootdevice",
        )
        .unwrap();
        assert_eq!(decoded.unique_device_name, "uuid:device-123");
        assert_eq!(decoded.usn_target.as_deref(), Some("upnp:rootdevice"));
        assert_eq!(decoded.max_age_seconds, 1800);
        assert_eq!(decoded.boot_id, Some(7));
        assert_eq!(decoded.config_id, Some(11));
    }

    #[test]
    fn ssdp_all_accepts_correlated_device_and_service_targets() {
        let decoded = decode_search_response(&response(""), "ssdp:all").unwrap();
        assert_eq!(decoded.search_target, "upnp:rootdevice");
    }

    #[test]
    fn rejects_duplicate_headers_bodies_and_target_mismatches() {
        let duplicate = String::from_utf8(response("ST: upnp:rootdevice\r\n")).unwrap();
        assert!(matches!(
            decode_search_response(duplicate.as_bytes(), "ssdp:all"),
            Err(SsdpError::DuplicateHeader(_))
        ));
        let mut body = response("");
        body.extend_from_slice(b"body");
        assert_eq!(
            decode_search_response(&body, "ssdp:all"),
            Err(SsdpError::MissingTerminator)
        );
        assert!(matches!(
            decode_search_response(&response(""), "urn:schemas-upnp-org:device:MediaServer:1"),
            Err(SsdpError::SearchTargetMismatch { .. })
        ));
    }

    #[test]
    fn rejects_missing_or_malformed_cache_and_usn_fields() {
        let invalid_cache = String::from_utf8(response(""))
            .unwrap()
            .replace("CACHE-CONTROL: max-age=1800", "CACHE-CONTROL: max-age=0");
        assert_eq!(
            decode_search_response(invalid_cache.as_bytes(), "ssdp:all"),
            Err(SsdpError::InvalidCacheControl)
        );
        let invalid_usn = String::from_utf8(response("")).unwrap().replace(
            "uuid:device-123::upnp:rootdevice",
            "device-123::upnp:rootdevice",
        );
        assert_eq!(
            decode_search_response(invalid_usn.as_bytes(), "ssdp:all"),
            Err(SsdpError::InvalidUsn)
        );

        let uuid_response = String::from_utf8(response(""))
            .unwrap()
            .replace("ST: upnp:rootdevice", "ST: uuid:other-device")
            .replace("::upnp:rootdevice", "");
        assert!(matches!(
            decode_search_response(uuid_response.as_bytes(), "ssdp:all"),
            Err(SsdpError::UsnTargetMismatch { .. })
        ));
    }
}
