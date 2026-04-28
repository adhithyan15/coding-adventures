//! Shared HTTP message types and helpers.
//!
//! Version-specific parsers disagree about wire syntax, but they should agree
//! about the semantic shapes that application code consumes. This crate
//! provides those shared shapes: headers, versions, request heads, response
//! heads, and body framing hints.

use std::fmt;

pub const VERSION: &str = "0.1.0";

/// A parsed route segment used for path matching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteSegment {
    Literal(String),
    Param(String),
}

/// A generic HTTP path pattern such as `/hello/:name`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutePattern {
    pub segments: Vec<RouteSegment>,
}

impl RoutePattern {
    pub fn parse(pattern: &str) -> Self {
        Self {
            segments: split_path_segments(pattern)
                .into_iter()
                .map(|segment| {
                    if let Some(name) = segment.strip_prefix(':') {
                        RouteSegment::Param(name.to_string())
                    } else {
                        RouteSegment::Literal(segment.to_string())
                    }
                })
                .collect(),
        }
    }

    pub fn match_path(&self, path: &str) -> Option<Vec<(String, String)>> {
        let path_segments = split_path_segments(path);
        if path_segments.len() != self.segments.len() {
            return None;
        }

        let mut params = Vec::new();
        for (segment, actual) in self.segments.iter().zip(path_segments) {
            match segment {
                RouteSegment::Literal(expected) if expected == actual => {}
                RouteSegment::Literal(_) => return None,
                RouteSegment::Param(name) => params.push((name.clone(), actual.to_string())),
            }
        }

        Some(params)
    }
}

/// One HTTP header line, preserved in arrival order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub name: String,
    pub value: String,
}

/// A semantic HTTP version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HttpVersion {
    pub major: u16,
    pub minor: u16,
}

impl HttpVersion {
    /// Parse a textual `HTTP/x.y` version marker.
    pub fn parse(text: &str) -> Result<Self, String> {
        let Some(rest) = text.strip_prefix("HTTP/") else {
            return Err(format!("invalid HTTP version: {text}"));
        };
        let Some((major_text, minor_text)) = rest.split_once('.') else {
            return Err(format!("invalid HTTP version: {text}"));
        };
        let major = major_text
            .parse::<u16>()
            .map_err(|_| format!("invalid HTTP version: {text}"))?;
        let minor = minor_text
            .parse::<u16>()
            .map_err(|_| format!("invalid HTTP version: {text}"))?;

        Ok(Self { major, minor })
    }
}

impl fmt::Display for HttpVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HTTP/{}.{}", self.major, self.minor)
    }
}

/// Describes how a caller should consume the payload bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BodyKind {
    None,
    ContentLength(usize),
    UntilEof,
    Chunked,
}

/// The semantic shape of a request head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestHead {
    pub method: String,
    pub target: String,
    pub version: HttpVersion,
    pub headers: Vec<Header>,
}

impl RequestHead {
    pub fn header(&self, name: &str) -> Option<&str> {
        find_header(&self.headers, name)
    }

    pub fn content_length(&self) -> Option<usize> {
        parse_content_length(&self.headers)
    }

    pub fn content_type(&self) -> Option<(String, Option<String>)> {
        parse_content_type(&self.headers)
    }
}

/// The semantic shape of a response head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseHead {
    pub version: HttpVersion,
    pub status: u16,
    pub reason: String,
    pub headers: Vec<Header>,
}

impl ResponseHead {
    pub fn header(&self, name: &str) -> Option<&str> {
        find_header(&self.headers, name)
    }

    pub fn content_length(&self) -> Option<usize> {
        parse_content_length(&self.headers)
    }

    pub fn content_type(&self) -> Option<(String, Option<String>)> {
        parse_content_type(&self.headers)
    }
}

/// Return the first matching header value using ASCII case-insensitive lookup.
pub fn find_header<'a>(headers: &'a [Header], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|header| header.name.eq_ignore_ascii_case(name))
        .map(|header| header.value.as_str())
}

/// Parse `Content-Length` when present and valid.
pub fn parse_content_length(headers: &[Header]) -> Option<usize> {
    let value = find_header(headers, "Content-Length")?;
    value.parse::<usize>().ok()
}

/// Split `Content-Type` into media type and optional charset.
pub fn parse_content_type(headers: &[Header]) -> Option<(String, Option<String>)> {
    let value = find_header(headers, "Content-Type")?;
    let mut pieces = value.split(';').map(str::trim);
    let media_type = pieces.next()?.to_string();
    if media_type.is_empty() {
        return None;
    }

    let charset = pieces.find_map(|piece| {
        let (name, raw_value) = piece.split_once('=')?;
        if name.trim().eq_ignore_ascii_case("charset") {
            Some(raw_value.trim().trim_matches('"').to_string())
        } else {
            None
        }
    });

    Some((media_type, charset))
}

/// Split an HTTP path or route pattern into slash-delimited segments.
pub fn split_path_segments(path: &str) -> Vec<&str> {
    if path == "/" {
        return Vec::new();
    }

    path.split('/').filter(|segment| !segment.is_empty()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_versions() {
        let version = HttpVersion::parse("HTTP/1.1").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 1);
        assert_eq!(version.to_string(), "HTTP/1.1");
    }

    #[test]
    fn finds_headers_case_insensitively() {
        let headers = vec![Header {
            name: "Content-Type".into(),
            value: "text/plain".into(),
        }];
        assert_eq!(find_header(&headers, "content-type"), Some("text/plain"));
    }

    #[test]
    fn parses_content_helpers() {
        let headers = vec![
            Header {
                name: "Content-Length".into(),
                value: "42".into(),
            },
            Header {
                name: "Content-Type".into(),
                value: "text/html; charset=utf-8".into(),
            },
        ];
        assert_eq!(parse_content_length(&headers), Some(42));
        assert_eq!(
            parse_content_type(&headers),
            Some(("text/html".into(), Some("utf-8".into())))
        );
    }

    #[test]
    fn heads_delegate_to_helpers() {
        let request = RequestHead {
            method: "POST".into(),
            target: "/submit".into(),
            version: HttpVersion { major: 1, minor: 1 },
            headers: vec![Header {
                name: "Content-Length".into(),
                value: "5".into(),
            }],
        };
        let response = ResponseHead {
            version: HttpVersion { major: 1, minor: 0 },
            status: 200,
            reason: "OK".into(),
            headers: vec![Header {
                name: "Content-Type".into(),
                value: "application/json".into(),
            }],
        };

        assert_eq!(request.content_length(), Some(5));
        assert_eq!(
            response.content_type(),
            Some(("application/json".into(), None))
        );
    }

    #[test]
    fn route_pattern_matches_named_params() {
        let pattern = RoutePattern::parse("/hello/:name");
        assert_eq!(
            pattern.match_path("/hello/Adhithya"),
            Some(vec![("name".into(), "Adhithya".into())])
        );
        assert_eq!(pattern.match_path("/hello"), None);
        assert_eq!(pattern.match_path("/goodbye/Adhithya"), None);
    }

    #[test]
    fn route_pattern_handles_root_paths() {
        let pattern = RoutePattern::parse("/");
        assert_eq!(pattern.match_path("/"), Some(Vec::new()));
        assert_eq!(pattern.match_path("/extra"), None);
    }
}
