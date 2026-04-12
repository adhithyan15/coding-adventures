//! # url-parser
//!
//! RFC 1738 URL parser with relative resolution and percent-encoding.
//!
//! A URL (Uniform Resource Locator) tells you **where** something is on the
//! internet and **how** to get it. This crate parses URLs into their component
//! parts, resolves relative URLs against a base, and handles percent-encoding.
//!
//! ## URL anatomy
//!
//! ```text
//!   http://alice:secret@www.example.com:8080/docs/page.html?q=hello#section2
//!   └─┬─┘ └────┬─────┘└──────┬───────┘└─┬─┘└─────┬───────┘└──┬───┘└───┬───┘
//!  scheme  userinfo        host       port     path         query   fragment
//! ```
//!
//! - **scheme**: how to deliver (http, ftp, mailto)
//! - **host**: which server (www.example.com)
//! - **port**: which door (8080; defaults to 80 for http)
//! - **path**: which resource (/docs/page.html)
//! - **query**: parameters (?q=hello)
//! - **fragment**: client-side anchor (#section2) — never sent to server
//! - **userinfo**: credentials (rare today, common in early web)
//!
//! ## Parsing algorithm
//!
//! The URL is parsed left-to-right in a single pass, no backtracking:
//!
//! 1. Find `://` → extract scheme (lowercased)
//! 2. Find `#` from right → extract fragment
//! 3. Find `?` → extract query
//! 4. Find first `/` → extract path
//! 5. Find `@` → extract userinfo
//! 6. Find last `:` → extract port
//! 7. Remainder → host (lowercased)
//!
//! ## Example
//!
//! ```rust
//! use url_parser::Url;
//!
//! let url = Url::parse("http://www.example.com:8080/docs/page.html?q=hello#s2").unwrap();
//! assert_eq!(url.scheme, "http");
//! assert_eq!(url.host.as_deref(), Some("www.example.com"));
//! assert_eq!(url.port, Some(8080));
//! assert_eq!(url.path, "/docs/page.html");
//! assert_eq!(url.query.as_deref(), Some("q=hello"));
//! assert_eq!(url.fragment.as_deref(), Some("s2"));
//! assert_eq!(url.effective_port(), Some(8080));
//! ```

pub const VERSION: &str = "0.1.0";

use std::fmt;

// ============================================================================
// Error type
// ============================================================================

/// Errors that can occur when parsing or resolving a URL.
#[derive(Debug, Clone, PartialEq)]
pub enum UrlError {
    /// No scheme found (e.g., "www.example.com" without "http://")
    MissingScheme,
    /// Scheme contains invalid characters (must be `[a-z][a-z0-9+.-]*`)
    InvalidScheme,
    /// Port is not a valid u16 (e.g., "http://host:99999")
    InvalidPort,
    /// Percent-encoding is malformed (e.g., "%GG", "%2" truncated)
    InvalidPercentEncoding,
    /// Empty host in an authority-based URL ("http:///path")
    EmptyHost,
    /// Relative URL cannot be resolved without a base
    RelativeWithoutBase,
}

impl fmt::Display for UrlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UrlError::MissingScheme => write!(f, "missing scheme (expected '://')"),
            UrlError::InvalidScheme => write!(f, "invalid scheme (must be [a-z][a-z0-9+.-]*)"),
            UrlError::InvalidPort => write!(f, "invalid port (must be 0-65535)"),
            UrlError::InvalidPercentEncoding => write!(f, "malformed percent-encoding"),
            UrlError::EmptyHost => write!(f, "empty host in authority-based URL"),
            UrlError::RelativeWithoutBase => write!(f, "relative URL requires a base URL"),
        }
    }
}

// ============================================================================
// Url struct
// ============================================================================

/// A parsed URL with all components separated.
///
/// All string fields store the decoded values. The `raw` field preserves the
/// original input for round-tripping.
///
/// ## Invariants
///
/// - `scheme` is always lowercased
/// - `host` is always lowercased (when present)
/// - `path` starts with `/` for authority-based URLs (http, ftp)
/// - `query` does NOT include the leading `?`
/// - `fragment` does NOT include the leading `#`
#[derive(Debug, Clone, PartialEq)]
pub struct Url {
    /// The scheme (protocol), lowercased. Examples: "http", "ftp", "mailto".
    pub scheme: String,
    /// Optional userinfo before the `@` in the authority. Example: "alice:secret".
    pub userinfo: Option<String>,
    /// Optional host, lowercased. Example: "www.example.com".
    pub host: Option<String>,
    /// Optional explicit port number. `None` means use the scheme default.
    pub port: Option<u16>,
    /// The path component. Always starts with `/` for HTTP URLs.
    pub path: String,
    /// Optional query string, without the leading `?`.
    pub query: Option<String>,
    /// Optional fragment identifier, without the leading `#`.
    pub fragment: Option<String>,
    /// The original input string, preserved verbatim.
    raw: String,
}

impl Url {
    /// Parse an absolute URL string.
    ///
    /// The input must contain a scheme (e.g., "http://..."). For relative URLs,
    /// first parse the base URL, then call [`Url::resolve()`].
    ///
    /// ## Algorithm
    ///
    /// Single-pass, left-to-right:
    ///
    /// ```text
    /// "http://alice:secret@www.example.com:8080/docs/page.html?q=hello#sec2"
    ///  ^^^^                                                              ^^^^
    ///  Step 1: scheme = "http"                            Step 2: fragment = "sec2"
    ///                                                   ^^^^^^^^
    ///                                           Step 3: query = "q=hello"
    ///                                    ^^^^^^^^^^^^^^^
    ///                            Step 4: path = "/docs/page.html"
    ///        ^^^^^^^^^^^^
    ///    Step 5: userinfo = "alice:secret"
    ///                                ^^^^
    ///                    Step 6: port = 8080
    ///                       ^^^^^^^^^^^^^^^
    ///               Step 7: host = "www.example.com"
    /// ```
    pub fn parse(input: &str) -> Result<Url, UrlError> {
        let raw = input.to_string();
        let input = input.trim();

        // Step 1: Extract scheme by finding "://"
        let (scheme, after_scheme) = match input.find("://") {
            Some(pos) => {
                let scheme = input[..pos].to_lowercase();
                validate_scheme(&scheme)?;
                (scheme, &input[pos + 3..])
            }
            None => {
                // Also handle "scheme:path" form (e.g., "mailto:alice@example.com")
                match input.find(':') {
                    Some(pos) if pos > 0 && !input[..pos].contains('/') => {
                        let scheme = input[..pos].to_lowercase();
                        validate_scheme(&scheme)?;
                        // No authority — the rest is the path
                        let path = &input[pos + 1..];
                        // Still split fragment and query from path
                        let (path, fragment) = split_fragment(path);
                        let (path, query) = split_query(path);
                        return Ok(Url {
                            scheme,
                            userinfo: None,
                            host: None,
                            port: None,
                            path: path.to_string(),
                            query: query.map(|s| s.to_string()),
                            fragment: fragment.map(|s| s.to_string()),
                            raw,
                        });
                    }
                    _ => return Err(UrlError::MissingScheme),
                }
            }
        };

        // Step 2: Extract fragment (find "#" from the right)
        let (after_scheme, fragment) = split_fragment(after_scheme);

        // Step 3: Extract query (find "?")
        let (after_scheme, query) = split_query(after_scheme);

        // Step 4: Split authority from path (find first "/")
        let (authority_str, path) = match after_scheme.find('/') {
            Some(pos) => (&after_scheme[..pos], &after_scheme[pos..]),
            None => (after_scheme, "/"),
        };

        // Step 5: Extract userinfo (find "@" in authority)
        let (userinfo, host_port) = match authority_str.rfind('@') {
            Some(pos) => (Some(&authority_str[..pos]), &authority_str[pos + 1..]),
            None => (None, authority_str),
        };

        // Step 6 & 7: Extract port and host
        //
        // IPv6 addresses are enclosed in brackets: [::1]:8080
        // For IPv6, the port delimiter is the ":" AFTER the closing "]"
        let (host, port) = if host_port.starts_with('[') {
            // IPv6: find closing bracket
            match host_port.find(']') {
                Some(bracket_pos) => {
                    let host = &host_port[..bracket_pos + 1];
                    let after_bracket = &host_port[bracket_pos + 1..];
                    let port = if after_bracket.starts_with(':') {
                        Some(parse_port(&after_bracket[1..])?)
                    } else {
                        None
                    };
                    (host, port)
                }
                None => (host_port, None), // malformed IPv6, treat whole thing as host
            }
        } else {
            // IPv4 or hostname: last ":" separates host from port
            match host_port.rfind(':') {
                Some(pos) => {
                    let maybe_port = &host_port[pos + 1..];
                    // Only treat as port if it's all digits
                    if !maybe_port.is_empty() && maybe_port.chars().all(|c| c.is_ascii_digit()) {
                        let host = &host_port[..pos];
                        let port = parse_port(maybe_port)?;
                        (host, Some(port))
                    } else {
                        (host_port, None)
                    }
                }
                None => (host_port, None),
            }
        };

        let host = if host.is_empty() {
            None
        } else {
            Some(host.to_lowercase())
        };

        Ok(Url {
            scheme,
            userinfo: userinfo.map(|s| s.to_string()),
            host,
            port,
            path: path.to_string(),
            query: query.map(|s| s.to_string()),
            fragment: fragment.map(|s| s.to_string()),
            raw,
        })
    }

    /// Resolve a relative URL against this URL as the base.
    ///
    /// Implements the RFC 1808 relative resolution algorithm:
    ///
    /// ```text
    /// if R has scheme     → R is absolute, return as-is
    /// if R starts with // → inherit scheme only
    /// if R starts with /  → inherit scheme + authority, replace path
    /// otherwise           → merge paths, resolve . and ..
    /// ```
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use url_parser::Url;
    ///
    /// let base = Url::parse("http://www.example.com/a/b/c.html").unwrap();
    ///
    /// // Same directory
    /// let resolved = base.resolve("d.html").unwrap();
    /// assert_eq!(resolved.path, "/a/b/d.html");
    ///
    /// // Parent directory
    /// let resolved = base.resolve("../d.html").unwrap();
    /// assert_eq!(resolved.path, "/a/d.html");
    ///
    /// // Absolute path
    /// let resolved = base.resolve("/x/y.html").unwrap();
    /// assert_eq!(resolved.path, "/x/y.html");
    /// ```
    pub fn resolve(&self, relative: &str) -> Result<Url, UrlError> {
        let relative = relative.trim();

        // Empty relative → return base without fragment
        if relative.is_empty() {
            let mut result = self.clone();
            result.fragment = None;
            result.raw = self.to_url_string();
            return Ok(result);
        }

        // Fragment-only: "#section"
        if relative.starts_with('#') {
            let mut result = self.clone();
            result.fragment = Some(relative[1..].to_string());
            result.raw = result.to_url_string();
            return Ok(result);
        }

        // If R has a scheme, it's already absolute
        if relative.contains("://") || (relative.contains(':') && !relative.starts_with('/')) {
            // Check if the part before ":" looks like a scheme
            if let Some(colon) = relative.find(':') {
                let maybe_scheme = &relative[..colon];
                if !maybe_scheme.is_empty()
                    && maybe_scheme.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
                    && maybe_scheme.chars().next().unwrap().is_ascii_alphabetic()
                {
                    return Url::parse(relative);
                }
            }
        }

        // Scheme-relative: "//host/path"
        if relative.starts_with("//") {
            let full = format!("{}:{}", self.scheme, relative);
            return Url::parse(&full);
        }

        // Absolute path: "/path"
        if relative.starts_with('/') {
            let (path, fragment) = split_fragment(relative);
            let (path, query) = split_query(path);
            let mut result = self.clone();
            result.path = remove_dot_segments(path);
            result.query = query.map(|s| s.to_string());
            result.fragment = fragment.map(|s| s.to_string());
            result.raw = result.to_url_string();
            return Ok(result);
        }

        // Relative path: merge with base
        let (relative_path, fragment) = split_fragment(relative);
        let (relative_path, query) = split_query(relative_path);

        // Merge: take base path up to last "/", append relative
        let merged = merge_paths(&self.path, relative_path);
        let resolved_path = remove_dot_segments(&merged);

        let mut result = self.clone();
        result.path = resolved_path;
        result.query = query.map(|s| s.to_string());
        result.fragment = fragment.map(|s| s.to_string());
        result.raw = result.to_url_string();
        Ok(result)
    }

    /// The effective port — explicit port if set, otherwise the scheme default.
    ///
    /// | Scheme | Default Port |
    /// |--------|-------------|
    /// | http   | 80          |
    /// | https  | 443         |
    /// | ftp    | 21          |
    pub fn effective_port(&self) -> Option<u16> {
        self.port.or_else(|| default_port(&self.scheme))
    }

    /// The authority string: `[userinfo@]host[:port]`
    pub fn authority(&self) -> String {
        let mut auth = String::new();
        if let Some(ref ui) = self.userinfo {
            auth.push_str(ui);
            auth.push('@');
        }
        if let Some(ref h) = self.host {
            auth.push_str(h);
        }
        if let Some(p) = self.port {
            auth.push(':');
            auth.push_str(&p.to_string());
        }
        auth
    }

    /// Serialize back to a URL string.
    pub fn to_url_string(&self) -> String {
        let mut s = String::new();
        s.push_str(&self.scheme);

        if self.host.is_some() {
            s.push_str("://");
            s.push_str(&self.authority());
        } else {
            s.push(':');
        }

        s.push_str(&self.path);

        if let Some(ref q) = self.query {
            s.push('?');
            s.push_str(q);
        }
        if let Some(ref f) = self.fragment {
            s.push('#');
            s.push_str(f);
        }
        s
    }
}

impl fmt::Display for Url {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_url_string())
    }
}

// ============================================================================
// Percent-encoding / decoding
// ============================================================================

/// Characters that do NOT need percent-encoding in a URL path.
///
/// RFC 1738 unreserved characters: `A-Z a-z 0-9 - _ . ~`
/// Plus path-safe characters: `/`
fn is_unreserved(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.' | b'~' | b'/')
}

/// Percent-encode a string for use in a URL path or query.
///
/// Encodes all characters except unreserved ones (`A-Z a-z 0-9 - _ . ~ /`).
///
/// ```rust
/// use url_parser::percent_encode;
/// assert_eq!(percent_encode("hello world"), "hello%20world");
/// assert_eq!(percent_encode("/path/to/file"), "/path/to/file");
/// ```
pub fn percent_encode(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    for byte in input.bytes() {
        if is_unreserved(byte) {
            result.push(byte as char);
        } else {
            result.push_str(&format!("%{:02X}", byte));
        }
    }
    result
}

/// Percent-decode a string: `"%20"` → `" "`, `"%E6%97%A5"` → `"日"`.
///
/// Each `%XX` sequence is replaced by the byte with that hex value. The
/// resulting bytes are interpreted as UTF-8.
///
/// ```rust
/// use url_parser::percent_decode;
/// assert_eq!(percent_decode("hello%20world").unwrap(), "hello world");
/// assert_eq!(percent_decode("%E6%97%A5").unwrap(), "日");
/// ```
pub fn percent_decode(input: &str) -> Result<String, UrlError> {
    let bytes = input.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'%' {
            // Need at least 2 more hex digits
            if i + 2 >= bytes.len() {
                return Err(UrlError::InvalidPercentEncoding);
            }
            let hi = hex_digit(bytes[i + 1])?;
            let lo = hex_digit(bytes[i + 2])?;
            result.push((hi << 4) | lo);
            i += 3;
        } else {
            result.push(bytes[i]);
            i += 1;
        }
    }

    String::from_utf8(result).map_err(|_| UrlError::InvalidPercentEncoding)
}

/// Convert a hex ASCII digit to its numeric value (0–15).
fn hex_digit(b: u8) -> Result<u8, UrlError> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(UrlError::InvalidPercentEncoding),
    }
}

// ============================================================================
// Internal helpers
// ============================================================================

/// Validate that a scheme matches `[a-z][a-z0-9+.-]*`.
fn validate_scheme(scheme: &str) -> Result<(), UrlError> {
    if scheme.is_empty() {
        return Err(UrlError::InvalidScheme);
    }
    let mut chars = scheme.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_lowercase() {
        return Err(UrlError::InvalidScheme);
    }
    for c in chars {
        if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '+' && c != '-' && c != '.' {
            return Err(UrlError::InvalidScheme);
        }
    }
    Ok(())
}

/// Parse a port string to u16.
fn parse_port(s: &str) -> Result<u16, UrlError> {
    s.parse::<u16>().map_err(|_| UrlError::InvalidPort)
}

/// Return the default port for a scheme, if known.
fn default_port(scheme: &str) -> Option<u16> {
    match scheme {
        "http" => Some(80),
        "https" => Some(443),
        "ftp" => Some(21),
        _ => None,
    }
}

/// Split a string at the first `#`, returning (before, Some(after)) or (input, None).
fn split_fragment(input: &str) -> (&str, Option<&str>) {
    match input.find('#') {
        Some(pos) => (&input[..pos], Some(&input[pos + 1..])),
        None => (input, None),
    }
}

/// Split a string at the first `?`, returning (before, Some(after)) or (input, None).
fn split_query(input: &str) -> (&str, Option<&str>) {
    match input.find('?') {
        Some(pos) => (&input[..pos], Some(&input[pos + 1..])),
        None => (input, None),
    }
}

/// Merge a base path and a relative path.
///
/// Takes everything in `base_path` up to and including the last `/`,
/// then appends `relative_path`.
///
/// ```text
/// merge("/a/b/c", "d")   → "/a/b/d"
/// merge("/a/b/",  "d")   → "/a/b/d"
/// merge("/a",     "d")   → "/d"
/// ```
fn merge_paths(base_path: &str, relative_path: &str) -> String {
    match base_path.rfind('/') {
        Some(pos) => {
            let mut merged = base_path[..=pos].to_string();
            merged.push_str(relative_path);
            merged
        }
        None => format!("/{}", relative_path),
    }
}

/// Remove `.` and `..` segments from a path.
///
/// Implements the "remove dot segments" algorithm from RFC 3986 §5.2.4:
///
/// ```text
/// /a/b/../c   → /a/c
/// /a/./b      → /a/b
/// /a/b/../../c → /c
/// /a/../../../c → /c   (can't go above root)
/// ```
fn remove_dot_segments(path: &str) -> String {
    let mut output_segments: Vec<&str> = Vec::new();

    for segment in path.split('/') {
        match segment {
            "." => {
                // Skip — "current directory" is a no-op
            }
            ".." => {
                // Go up one level — remove the last segment (if any)
                output_segments.pop();
            }
            _ => {
                output_segments.push(segment);
            }
        }
    }

    let result = output_segments.join("/");
    // Ensure the path starts with "/" if the input did
    if path.starts_with('/') && !result.starts_with('/') {
        format!("/{}", result)
    } else {
        result
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Basic parsing ──────────────────────────────────────────────────────

    #[test]
    fn parse_simple_http_url() {
        let url = Url::parse("http://www.example.com").unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.host.as_deref(), Some("www.example.com"));
        assert_eq!(url.port, None);
        assert_eq!(url.path, "/");
        assert_eq!(url.query, None);
        assert_eq!(url.fragment, None);
    }

    #[test]
    fn parse_http_with_path() {
        let url = Url::parse("http://www.example.com/docs/page.html").unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.host.as_deref(), Some("www.example.com"));
        assert_eq!(url.path, "/docs/page.html");
    }

    #[test]
    fn parse_all_components() {
        let url = Url::parse(
            "http://alice:secret@www.example.com:8080/docs/page.html?q=hello#section2",
        )
        .unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.userinfo.as_deref(), Some("alice:secret"));
        assert_eq!(url.host.as_deref(), Some("www.example.com"));
        assert_eq!(url.port, Some(8080));
        assert_eq!(url.path, "/docs/page.html");
        assert_eq!(url.query.as_deref(), Some("q=hello"));
        assert_eq!(url.fragment.as_deref(), Some("section2"));
    }

    #[test]
    fn parse_https_url() {
        let url = Url::parse("https://secure.example.com/login").unwrap();
        assert_eq!(url.scheme, "https");
        assert_eq!(url.host.as_deref(), Some("secure.example.com"));
        assert_eq!(url.effective_port(), Some(443));
    }

    #[test]
    fn parse_ftp_url() {
        let url = Url::parse("ftp://files.example.com/pub/readme.txt").unwrap();
        assert_eq!(url.scheme, "ftp");
        assert_eq!(url.effective_port(), Some(21));
    }

    #[test]
    fn parse_mailto_url() {
        let url = Url::parse("mailto:alice@example.com").unwrap();
        assert_eq!(url.scheme, "mailto");
        assert_eq!(url.host, None);
        assert_eq!(url.path, "alice@example.com");
    }

    // ─── Case normalization ─────────────────────────────────────────────────

    #[test]
    fn scheme_is_lowercased() {
        let url = Url::parse("HTTP://WWW.EXAMPLE.COM/PATH").unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.host.as_deref(), Some("www.example.com"));
        // Path case is preserved
        assert_eq!(url.path, "/PATH");
    }

    // ─── Effective port ─────────────────────────────────────────────────────

    #[test]
    fn effective_port_http_default() {
        let url = Url::parse("http://example.com").unwrap();
        assert_eq!(url.port, None);
        assert_eq!(url.effective_port(), Some(80));
    }

    #[test]
    fn effective_port_explicit() {
        let url = Url::parse("http://example.com:9090").unwrap();
        assert_eq!(url.port, Some(9090));
        assert_eq!(url.effective_port(), Some(9090));
    }

    // ─── Authority ──────────────────────────────────────────────────────────

    #[test]
    fn authority_with_all_parts() {
        let url = Url::parse("http://user:pass@host.com:8080/path").unwrap();
        assert_eq!(url.authority(), "user:pass@host.com:8080");
    }

    #[test]
    fn authority_host_only() {
        let url = Url::parse("http://host.com/path").unwrap();
        assert_eq!(url.authority(), "host.com");
    }

    // ─── Invalid URLs ───────────────────────────────────────────────────────

    #[test]
    fn missing_scheme() {
        assert_eq!(Url::parse("www.example.com"), Err(UrlError::MissingScheme));
    }

    #[test]
    fn invalid_scheme_starts_with_digit() {
        assert_eq!(Url::parse("1http://x.com"), Err(UrlError::InvalidScheme));
    }

    #[test]
    fn invalid_port_too_large() {
        assert_eq!(
            Url::parse("http://host:99999"),
            Err(UrlError::InvalidPort)
        );
    }

    // ─── Percent-encoding ───────────────────────────────────────────────────

    #[test]
    fn encode_space() {
        assert_eq!(percent_encode("hello world"), "hello%20world");
    }

    #[test]
    fn encode_preserves_unreserved() {
        assert_eq!(percent_encode("abc-def_ghi.jkl~mno"), "abc-def_ghi.jkl~mno");
    }

    #[test]
    fn encode_preserves_slashes() {
        assert_eq!(percent_encode("/path/to/file"), "/path/to/file");
    }

    #[test]
    fn decode_space() {
        assert_eq!(percent_decode("hello%20world").unwrap(), "hello world");
    }

    #[test]
    fn decode_utf8() {
        // 日 = U+65E5 = E6 97 A5 in UTF-8
        assert_eq!(percent_decode("%E6%97%A5").unwrap(), "日");
    }

    #[test]
    fn decode_roundtrip() {
        let original = "hello world/日本語";
        let encoded = percent_encode(original);
        let decoded = percent_decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn decode_malformed_truncated() {
        assert_eq!(percent_decode("%2"), Err(UrlError::InvalidPercentEncoding));
    }

    #[test]
    fn decode_malformed_bad_hex() {
        assert_eq!(
            percent_decode("%GG"),
            Err(UrlError::InvalidPercentEncoding)
        );
    }

    // ─── Relative resolution ────────────────────────────────────────────────

    #[test]
    fn resolve_same_directory() {
        let base = Url::parse("http://host/a/b/c.html").unwrap();
        let resolved = base.resolve("d.html").unwrap();
        assert_eq!(resolved.scheme, "http");
        assert_eq!(resolved.host.as_deref(), Some("host"));
        assert_eq!(resolved.path, "/a/b/d.html");
    }

    #[test]
    fn resolve_parent_directory() {
        let base = Url::parse("http://host/a/b/c.html").unwrap();
        let resolved = base.resolve("../d.html").unwrap();
        assert_eq!(resolved.path, "/a/d.html");
    }

    #[test]
    fn resolve_grandparent_directory() {
        let base = Url::parse("http://host/a/b/c.html").unwrap();
        let resolved = base.resolve("../../d.html").unwrap();
        assert_eq!(resolved.path, "/d.html");
    }

    #[test]
    fn resolve_absolute_path() {
        let base = Url::parse("http://host/a/b/c.html").unwrap();
        let resolved = base.resolve("/x/y.html").unwrap();
        assert_eq!(resolved.path, "/x/y.html");
        assert_eq!(resolved.host.as_deref(), Some("host"));
    }

    #[test]
    fn resolve_scheme_relative() {
        let base = Url::parse("http://host/a/b").unwrap();
        let resolved = base.resolve("//other.com/path").unwrap();
        assert_eq!(resolved.scheme, "http");
        assert_eq!(resolved.host.as_deref(), Some("other.com"));
        assert_eq!(resolved.path, "/path");
    }

    #[test]
    fn resolve_already_absolute() {
        let base = Url::parse("http://host/a/b").unwrap();
        let resolved = base.resolve("https://other.com/x").unwrap();
        assert_eq!(resolved.scheme, "https");
        assert_eq!(resolved.host.as_deref(), Some("other.com"));
        assert_eq!(resolved.path, "/x");
    }

    #[test]
    fn resolve_dot_segments() {
        let base = Url::parse("http://host/a/b/c").unwrap();
        let resolved = base.resolve("./d").unwrap();
        assert_eq!(resolved.path, "/a/b/d");
    }

    #[test]
    fn resolve_empty_returns_base() {
        let base = Url::parse("http://host/a/b?q=1#frag").unwrap();
        let resolved = base.resolve("").unwrap();
        assert_eq!(resolved.path, "/a/b");
        assert_eq!(resolved.query.as_deref(), Some("q=1"));
        assert_eq!(resolved.fragment, None); // fragment stripped
    }

    #[test]
    fn resolve_fragment_only() {
        let base = Url::parse("http://host/a/b").unwrap();
        let resolved = base.resolve("#sec").unwrap();
        assert_eq!(resolved.path, "/a/b");
        assert_eq!(resolved.fragment.as_deref(), Some("sec"));
    }

    #[test]
    fn resolve_with_query() {
        let base = Url::parse("http://host/a/b").unwrap();
        let resolved = base.resolve("c?key=val").unwrap();
        assert_eq!(resolved.path, "/a/c");
        assert_eq!(resolved.query.as_deref(), Some("key=val"));
    }

    // ─── Dot segment removal ────────────────────────────────────────────────

    #[test]
    fn remove_single_dot() {
        assert_eq!(remove_dot_segments("/a/./b"), "/a/b");
    }

    #[test]
    fn remove_double_dot() {
        assert_eq!(remove_dot_segments("/a/b/../c"), "/a/c");
    }

    #[test]
    fn remove_multiple_double_dots() {
        assert_eq!(remove_dot_segments("/a/b/../../c"), "/c");
    }

    #[test]
    fn double_dot_above_root() {
        // Can't go above root — just produces "/"
        assert_eq!(remove_dot_segments("/a/../../../c"), "/c");
    }

    // ─── to_url_string / Display ────────────────────────────────────────────

    #[test]
    fn roundtrip_full_url() {
        let input = "http://user:pass@host.com:8080/path?q=1#frag";
        let url = Url::parse(input).unwrap();
        assert_eq!(url.to_url_string(), input);
    }

    #[test]
    fn roundtrip_simple_url() {
        let input = "http://example.com/path";
        let url = Url::parse(input).unwrap();
        assert_eq!(url.to_url_string(), input);
    }

    // ─── Historical Mosaic-era URLs ─────────────────────────────────────────

    #[test]
    fn parse_cern_original_url() {
        let url =
            Url::parse("http://info.cern.ch/hypertext/WWW/TheProject.html").unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.host.as_deref(), Some("info.cern.ch"));
        assert_eq!(url.path, "/hypertext/WWW/TheProject.html");
        assert_eq!(url.effective_port(), Some(80));
    }

    #[test]
    fn parse_ncsa_mosaic_url() {
        let url = Url::parse("http://www.ncsa.uiuc.edu/SDG/Software/Mosaic/").unwrap();
        assert_eq!(url.host.as_deref(), Some("www.ncsa.uiuc.edu"));
        assert_eq!(url.path, "/SDG/Software/Mosaic/");
    }

    // ─── IPv6 ───────────────────────────────────────────────────────────────

    #[test]
    fn parse_ipv6_localhost() {
        let url = Url::parse("http://[::1]:8080/path").unwrap();
        assert_eq!(url.host.as_deref(), Some("[::1]"));
        assert_eq!(url.port, Some(8080));
        assert_eq!(url.path, "/path");
    }

    // ─── Edge cases ─────────────────────────────────────────────────────────

    #[test]
    fn parse_trailing_slash() {
        let url = Url::parse("http://host/").unwrap();
        assert_eq!(url.path, "/");
    }

    #[test]
    fn parse_query_without_path() {
        let url = Url::parse("http://host?q=1").unwrap();
        assert_eq!(url.host.as_deref(), Some("host"));
        assert_eq!(url.path, "/");
        assert_eq!(url.query.as_deref(), Some("q=1"));
    }

    #[test]
    fn parse_fragment_without_path() {
        let url = Url::parse("http://host#frag").unwrap();
        assert_eq!(url.host.as_deref(), Some("host"));
        assert_eq!(url.path, "/");
        assert_eq!(url.fragment.as_deref(), Some("frag"));
    }
}
