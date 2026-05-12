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

    /// Match against a full request target such as `/clip/v2/resource?limit=10`.
    ///
    /// Route matching uses only the path portion so query strings cannot make
    /// an otherwise-valid local API route miss.
    pub fn match_target(&self, target: &str) -> Option<Vec<(String, String)>> {
        self.match_path(parse_request_target(target).path)
    }
}

/// One HTTP header line, preserved in arrival order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub name: String,
    pub value: String,
}

/// Borrowed view of an origin-form request target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestTarget<'a> {
    pub path: &'a str,
    pub query: Option<&'a str>,
    pub fragment: Option<&'a str>,
}

impl<'a> RequestTarget<'a> {
    pub fn query_pairs(&self) -> QueryPairs<'a> {
        QueryPairs {
            rest: self.query.unwrap_or(""),
        }
    }

    pub fn query_value(&self, name: &str) -> Option<&'a str> {
        self.query_pairs()
            .find(|(candidate, _)| candidate == &name)
            .map(|(_, value)| value)
    }
}

/// Iterator over raw `name=value` pairs in a query string.
///
/// Values are intentionally not percent-decoded here. That keeps this crate a
/// syntax-level core that can feed callers with different decoding policies.
#[derive(Debug, Clone)]
pub struct QueryPairs<'a> {
    rest: &'a str,
}

impl<'a> Iterator for QueryPairs<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        while !self.rest.is_empty() {
            let (piece, next_rest) = match self.rest.split_once('&') {
                Some((piece, rest)) => (piece, rest),
                None => (self.rest, ""),
            };
            self.rest = next_rest;
            if piece.is_empty() {
                continue;
            }

            return Some(match piece.split_once('=') {
                Some((name, value)) => (name, value),
                None => (piece, ""),
            });
        }
        None
    }
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

    pub fn target_parts(&self) -> RequestTarget<'_> {
        parse_request_target(&self.target)
    }

    pub fn path(&self) -> &str {
        self.target_parts().path
    }

    pub fn query_value(&self, name: &str) -> Option<&str> {
        self.target_parts().query_value(name)
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

/// Split an origin-form HTTP request target into path, query, and fragment.
pub fn parse_request_target(target: &str) -> RequestTarget<'_> {
    let (before_fragment, fragment) = match target.split_once('#') {
        Some((head, fragment)) => (head, Some(fragment)),
        None => (target, None),
    };
    let (path, query) = match before_fragment.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (before_fragment, None),
    };

    RequestTarget {
        path: if path.is_empty() { "/" } else { path },
        query,
        fragment,
    }
}

/// Split an HTTP path or route pattern into slash-delimited segments.
pub fn split_path_segments(path: &str) -> Vec<&str> {
    if path == "/" {
        return Vec::new();
    }

    path.split('/')
        .filter(|segment| !segment.is_empty())
        .collect()
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
    fn parses_request_targets_without_decoding_query_values() {
        let target = parse_request_target("/clip/v2/resource/light?id=abc%20123&limit=10#ignored");
        assert_eq!(target.path, "/clip/v2/resource/light");
        assert_eq!(target.query, Some("id=abc%20123&limit=10"));
        assert_eq!(target.fragment, Some("ignored"));
        assert_eq!(
            target.query_pairs().collect::<Vec<_>>(),
            vec![("id", "abc%20123"), ("limit", "10")]
        );
        assert_eq!(target.query_value("limit"), Some("10"));
        assert_eq!(target.query_value("missing"), None);
    }

    #[test]
    fn request_heads_expose_path_and_query_helpers() {
        let request = RequestHead {
            method: "GET".into(),
            target: "/api/devices?room=kitchen&verbose".into(),
            version: HttpVersion { major: 1, minor: 1 },
            headers: Vec::new(),
        };

        assert_eq!(request.path(), "/api/devices");
        assert_eq!(request.query_value("room"), Some("kitchen"));
        assert_eq!(request.query_value("verbose"), Some(""));
    }

    #[test]
    fn route_patterns_match_request_targets_by_path_only() {
        let pattern = RoutePattern::parse("/clip/v2/resource/:kind/:id");
        assert_eq!(
            pattern.match_target("/clip/v2/resource/light/abc?limit=10"),
            Some(vec![
                ("kind".to_string(), "light".to_string()),
                ("id".to_string(), "abc".to_string()),
            ])
        );
        assert_eq!(pattern.match_target("/clip/v2/resource/light"), None);
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
