//! HTTP/1 request and response head parsing.
//!
//! HTTP/1 is text-framed. A caller first needs the start line and headers, then
//! it needs clear instructions for how to consume the body bytes that follow.
//! This crate focuses on exactly that boundary.

use http_core::{BodyKind, Header, HttpVersion, RequestHead, ResponseHead};
use std::fmt;

pub const VERSION: &str = "0.1.0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedRequestHead {
    pub head: RequestHead,
    pub body_offset: usize,
    pub body_kind: BodyKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedResponseHead {
    pub head: ResponseHead,
    pub body_offset: usize,
    pub body_kind: BodyKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Http1ParseError {
    IncompleteHead,
    InvalidHeadEncoding,
    InvalidStartLine(String),
    InvalidHeader(String),
    InvalidVersion(String),
    InvalidStatus(String),
    InvalidContentLength(String),
}

impl fmt::Display for Http1ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompleteHead => write!(f, "incomplete HTTP/1 head"),
            Self::InvalidHeadEncoding => write!(f, "invalid HTTP/1 head encoding"),
            Self::InvalidStartLine(line) => write!(f, "invalid HTTP/1 start line: {line}"),
            Self::InvalidHeader(line) => write!(f, "invalid HTTP/1 header: {line}"),
            Self::InvalidVersion(value) => write!(f, "invalid HTTP version: {value}"),
            Self::InvalidStatus(value) => write!(f, "invalid HTTP status: {value}"),
            Self::InvalidContentLength(value) => write!(f, "invalid Content-Length: {value}"),
        }
    }
}

impl std::error::Error for Http1ParseError {}

pub fn parse_request_head(input: &[u8]) -> Result<ParsedRequestHead, Http1ParseError> {
    let (lines, body_offset) = split_head_lines(input)?;
    let (start_line, header_lines) = lines
        .split_first()
        .ok_or_else(|| Http1ParseError::InvalidStartLine(String::new()))?;
    let start_line =
        std::str::from_utf8(start_line).map_err(|_| Http1ParseError::InvalidHeadEncoding)?;
    let mut parts = start_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| Http1ParseError::InvalidStartLine(start_line.to_string()))?;
    let target = parts
        .next()
        .ok_or_else(|| Http1ParseError::InvalidStartLine(start_line.to_string()))?;
    let version_text = parts
        .next()
        .ok_or_else(|| Http1ParseError::InvalidStartLine(start_line.to_string()))?;
    if parts.next().is_some() {
        return Err(Http1ParseError::InvalidStartLine(start_line.to_string()));
    }

    let version = HttpVersion::parse(version_text)
        .map_err(|_| Http1ParseError::InvalidVersion(version_text.into()))?;
    let headers = parse_headers(header_lines)?;
    let body_kind = request_body_kind(&headers)?;

    Ok(ParsedRequestHead {
        head: RequestHead {
            method: method.into(),
            target: target.into(),
            version,
            headers,
        },
        body_offset,
        body_kind,
    })
}

pub fn parse_response_head(input: &[u8]) -> Result<ParsedResponseHead, Http1ParseError> {
    let (lines, body_offset) = split_head_lines(input)?;
    let (status_line, header_lines) = lines
        .split_first()
        .ok_or_else(|| Http1ParseError::InvalidStartLine(String::new()))?;
    let status_line =
        std::str::from_utf8(status_line).map_err(|_| Http1ParseError::InvalidHeadEncoding)?;
    let pieces: Vec<&str> = status_line.split_whitespace().collect();
    if pieces.len() < 2 {
        return Err(Http1ParseError::InvalidStartLine(status_line.to_string()));
    }

    let version = HttpVersion::parse(pieces[0])
        .map_err(|_| Http1ParseError::InvalidVersion(pieces[0].into()))?;
    let status = pieces[1]
        .parse::<u16>()
        .map_err(|_| Http1ParseError::InvalidStatus(pieces[1].into()))?;
    let reason = if pieces.len() > 2 {
        pieces[2..].join(" ")
    } else {
        String::new()
    };

    let headers = parse_headers(header_lines)?;
    let body_kind = response_body_kind(status, &headers)?;

    Ok(ParsedResponseHead {
        head: ResponseHead {
            version,
            status,
            reason,
            headers,
        },
        body_offset,
        body_kind,
    })
}

fn split_head_lines(input: &[u8]) -> Result<(Vec<&[u8]>, usize), Http1ParseError> {
    let mut index = 0;
    while index < input.len() {
        if input[index..].starts_with(b"\r\n") {
            index += 2;
        } else if input[index] == b'\n' {
            index += 1;
        } else {
            break;
        }
    }

    let mut lines = Vec::new();
    loop {
        if index >= input.len() {
            return Err(Http1ParseError::IncompleteHead);
        }

        let line_start = index;
        while index < input.len() && input[index] != b'\n' {
            index += 1;
        }
        if index >= input.len() {
            return Err(Http1ParseError::IncompleteHead);
        }

        let line_end = if index > line_start && input[index - 1] == b'\r' {
            index - 1
        } else {
            index
        };
        let line = &input[line_start..line_end];
        index += 1;

        if line.is_empty() {
            return Ok((lines, index));
        }
        lines.push(line);
    }
}

fn parse_headers(lines: &[&[u8]]) -> Result<Vec<Header>, Http1ParseError> {
    let mut headers = Vec::with_capacity(lines.len());
    for line in lines {
        let text = std::str::from_utf8(line).map_err(|_| Http1ParseError::InvalidHeadEncoding)?;
        let (name, raw_value) = text
            .split_once(':')
            .ok_or_else(|| Http1ParseError::InvalidHeader(text.to_string()))?;
        let name = name.trim();
        if name.is_empty() {
            return Err(Http1ParseError::InvalidHeader(text.to_string()));
        }
        headers.push(Header {
            name: name.into(),
            value: raw_value.trim_matches([' ', '\t']).into(),
        });
    }
    Ok(headers)
}

fn request_body_kind(headers: &[Header]) -> Result<BodyKind, Http1ParseError> {
    if has_chunked_transfer_encoding(headers) {
        return Ok(BodyKind::Chunked);
    }

    match declared_content_length(headers)? {
        Some(0) | None => Ok(BodyKind::None),
        Some(length) => Ok(BodyKind::ContentLength(length)),
    }
}

fn response_body_kind(status: u16, headers: &[Header]) -> Result<BodyKind, Http1ParseError> {
    if (100..200).contains(&status) || status == 204 || status == 304 {
        return Ok(BodyKind::None);
    }
    if has_chunked_transfer_encoding(headers) {
        return Ok(BodyKind::Chunked);
    }

    match declared_content_length(headers)? {
        Some(0) => Ok(BodyKind::None),
        Some(length) => Ok(BodyKind::ContentLength(length)),
        None => Ok(BodyKind::UntilEof),
    }
}

fn declared_content_length(headers: &[Header]) -> Result<Option<usize>, Http1ParseError> {
    let Some(value) = headers
        .iter()
        .find(|header| header.name.eq_ignore_ascii_case("Content-Length"))
        .map(|header| header.value.as_str())
    else {
        return Ok(None);
    };

    value
        .parse::<usize>()
        .map(Some)
        .map_err(|_| Http1ParseError::InvalidContentLength(value.into()))
}

fn has_chunked_transfer_encoding(headers: &[Header]) -> bool {
    headers
        .iter()
        .filter(|header| header.name.eq_ignore_ascii_case("Transfer-Encoding"))
        .flat_map(|header| header.value.split(','))
        .any(|value| value.trim().eq_ignore_ascii_case("chunked"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_core::BodyKind;

    #[test]
    fn parses_simple_request() {
        let parsed = parse_request_head(b"GET / HTTP/1.0\r\nHost: example.com\r\n\r\n").unwrap();
        assert_eq!(parsed.head.method, "GET");
        assert_eq!(parsed.head.target, "/");
        assert_eq!(parsed.head.version, HttpVersion { major: 1, minor: 0 });
        assert_eq!(parsed.head.headers.len(), 1);
        assert_eq!(parsed.body_kind, BodyKind::None);
    }

    #[test]
    fn parses_content_length_request() {
        let parsed =
            parse_request_head(b"POST /submit HTTP/1.1\r\nContent-Length: 5\r\n\r\nhello").unwrap();
        assert_eq!(parsed.body_offset, 44);
        assert_eq!(parsed.body_kind, BodyKind::ContentLength(5));
    }

    #[test]
    fn parses_response_and_reason() {
        let parsed =
            parse_response_head(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nbody").unwrap();
        assert_eq!(parsed.head.status, 200);
        assert_eq!(parsed.head.reason, "OK");
        assert_eq!(parsed.body_kind, BodyKind::ContentLength(4));
    }

    #[test]
    fn response_without_length_reads_until_eof() {
        let parsed = parse_response_head(b"HTTP/1.0 200 OK\r\nServer: Venture\r\n\r\n").unwrap();
        assert_eq!(parsed.body_kind, BodyKind::UntilEof);
    }

    #[test]
    fn bodyless_status_codes_override_headers() {
        let parsed =
            parse_response_head(b"HTTP/1.1 204 No Content\r\nContent-Length: 12\r\n\r\n").unwrap();
        assert_eq!(parsed.body_kind, BodyKind::None);
    }

    #[test]
    fn accepts_lf_only_lines_and_duplicate_headers() {
        let parsed =
            parse_response_head(b"\nHTTP/1.1 200 OK\nSet-Cookie: a=1\nSet-Cookie: b=2\n\npayload")
                .unwrap();
        assert_eq!(parsed.head.headers.len(), 2);
        assert_eq!(parsed.head.headers[0].value, "a=1");
        assert_eq!(parsed.head.headers[1].value, "b=2");
    }

    #[test]
    fn rejects_invalid_headers() {
        let error = parse_request_head(b"GET / HTTP/1.1\r\nHost example.com\r\n\r\n").unwrap_err();
        assert!(matches!(error, Http1ParseError::InvalidHeader(_)));
    }

    #[test]
    fn rejects_invalid_content_length() {
        let error =
            parse_response_head(b"HTTP/1.1 200 OK\r\nContent-Length: nope\r\n\r\n").unwrap_err();
        assert!(matches!(error, Http1ParseError::InvalidContentLength(_)));
    }
}
