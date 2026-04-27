//! # url-parser-wasm
//!
//! WebAssembly bindings for the Rust `url-parser` crate via `wasm-bindgen`.
//!
//! This thin wrapper exposes the URL parser to JavaScript running in a browser
//! or Node.js through WebAssembly. Each method maps directly to the underlying
//! Rust implementation — no new logic is introduced here.
//!
//! ## Architecture
//!
//! ```text
//!   JavaScript  ──wasm-bindgen──►  WasmUrl (this crate)  ──►  url_parser::Url
//!   (browser)                      (thin adapter)              (all the work)
//! ```

use url_parser::{self, Url};
use wasm_bindgen::prelude::*;

// ============================================================================
// Helper: convert UrlError → JsValue for wasm-bindgen error propagation
// ============================================================================

fn to_js_error(err: url_parser::UrlError) -> JsValue {
    JsValue::from_str(&err.to_string())
}

// ============================================================================
// WasmUrl — the wasm-bindgen exported struct
// ============================================================================

/// A parsed URL exposed to JavaScript via WebAssembly.
///
/// Wraps `url_parser::Url` and exposes each field as a getter plus methods
/// for resolution, serialization, and port lookup.
#[wasm_bindgen]
pub struct WasmUrl {
    inner: Url,
}

#[wasm_bindgen]
impl WasmUrl {
    // ── Constructor: parse a URL string ────────────────────────────────

    /// Parse an absolute URL string into its components.
    ///
    /// Throws a string error if the URL is malformed (missing scheme,
    /// invalid port, etc.).
    #[wasm_bindgen(constructor)]
    pub fn new(input: &str) -> Result<WasmUrl, JsValue> {
        let url = Url::parse(input).map_err(to_js_error)?;
        Ok(WasmUrl { inner: url })
    }

    // ── Getters for each URL component ─────────────────────────────────

    /// The scheme (protocol), lowercased. Examples: "http", "ftp", "mailto".
    #[wasm_bindgen(getter)]
    pub fn scheme(&self) -> String {
        self.inner.scheme.clone()
    }

    /// Optional userinfo before the `@`. Returns empty string if absent.
    #[wasm_bindgen(getter)]
    pub fn userinfo(&self) -> Option<String> {
        self.inner.userinfo.clone()
    }

    /// Optional host, lowercased. Returns None if absent (e.g., mailto: URLs).
    #[wasm_bindgen(getter)]
    pub fn host(&self) -> Option<String> {
        self.inner.host.clone()
    }

    /// Optional explicit port number. Returns None if not specified.
    #[wasm_bindgen(getter)]
    pub fn port(&self) -> Option<u16> {
        self.inner.port
    }

    /// The path component. Always starts with `/` for HTTP URLs.
    #[wasm_bindgen(getter)]
    pub fn path(&self) -> String {
        self.inner.path.clone()
    }

    /// Optional query string, without the leading `?`.
    #[wasm_bindgen(getter)]
    pub fn query(&self) -> Option<String> {
        self.inner.query.clone()
    }

    /// Optional fragment identifier, without the leading `#`.
    #[wasm_bindgen(getter)]
    pub fn fragment(&self) -> Option<String> {
        self.inner.fragment.clone()
    }

    // ── Methods ────────────────────────────────────────────────────────

    /// Resolve a relative URL against this URL as the base.
    pub fn resolve(&self, relative: &str) -> Result<WasmUrl, JsValue> {
        let resolved = self.inner.resolve(relative).map_err(to_js_error)?;
        Ok(WasmUrl { inner: resolved })
    }

    /// The effective port — explicit port or scheme default (80/443/21).
    #[wasm_bindgen(js_name = "effectivePort")]
    pub fn effective_port(&self) -> Option<u16> {
        self.inner.effective_port()
    }

    /// The authority string: `[userinfo@]host[:port]`.
    pub fn authority(&self) -> String {
        self.inner.authority()
    }

    /// Serialize back to a URL string.
    #[wasm_bindgen(js_name = "toUrlString")]
    pub fn to_url_string(&self) -> String {
        self.inner.to_url_string()
    }

    /// Alias for `toUrlString()` — used by JavaScript's `String()` coercion.
    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string_js(&self) -> String {
        self.inner.to_url_string()
    }
}

// ============================================================================
// Free functions — percent-encoding utilities
// ============================================================================

/// Percent-encode a string for use in a URL path or query.
///
/// Unreserved characters (`A-Za-z0-9-_.~/`) pass through unchanged;
/// everything else becomes `%XX` (uppercase hex).
#[wasm_bindgen(js_name = "percentEncode")]
pub fn percent_encode(input: &str) -> String {
    url_parser::percent_encode(input)
}

/// Percent-decode a string: `"%20"` → `" "`, `"%E6%97%A5"` → `"日"`.
///
/// Throws if the encoding is malformed (truncated `%2` or non-hex `%GG`).
#[wasm_bindgen(js_name = "percentDecode")]
pub fn percent_decode(input: &str) -> Result<String, JsValue> {
    url_parser::percent_decode(input).map_err(to_js_error)
}

// ============================================================================
// Tests — verify the wasm adapter layer works (runs as native Rust tests)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_url() {
        let url = WasmUrl::new("http://example.com/path").unwrap();
        assert_eq!(url.scheme(), "http");
        assert_eq!(url.host(), Some("example.com".to_string()));
        assert_eq!(url.path(), "/path");
        assert_eq!(url.port(), None);
        assert_eq!(url.effective_port(), Some(80));
    }

    #[test]
    fn parse_all_components() {
        let url = WasmUrl::new("http://user:pass@host.com:8080/p?q=1#f").unwrap();
        assert_eq!(url.scheme(), "http");
        assert_eq!(url.userinfo(), Some("user:pass".to_string()));
        assert_eq!(url.host(), Some("host.com".to_string()));
        assert_eq!(url.port(), Some(8080));
        assert_eq!(url.path(), "/p");
        assert_eq!(url.query(), Some("q=1".to_string()));
        assert_eq!(url.fragment(), Some("f".to_string()));
        assert_eq!(url.authority(), "user:pass@host.com:8080");
    }

    #[test]
    fn parse_mailto() {
        let url = WasmUrl::new("mailto:alice@example.com").unwrap();
        assert_eq!(url.scheme(), "mailto");
        assert_eq!(url.host(), None);
        assert_eq!(url.path(), "alice@example.com");
    }

    #[test]
    fn resolve_relative() {
        let base = WasmUrl::new("http://host/a/b/c.html").unwrap();
        let resolved = base.resolve("../d.html").unwrap();
        assert_eq!(resolved.path(), "/a/d.html");
        assert_eq!(resolved.host(), Some("host".to_string()));
    }

    #[test]
    fn roundtrip() {
        let input = "http://user:pass@host.com:8080/path?q=1#frag";
        let url = WasmUrl::new(input).unwrap();
        assert_eq!(url.to_url_string(), input);
    }

    #[test]
    fn percent_encode_decode_roundtrip() {
        let original = "hello world/日本語";
        let encoded = percent_encode(original);
        let decoded = percent_decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    // Error-path tests use the underlying Rust crate directly, because
    // wasm-bindgen's JsValue panics on non-wasm32 targets. The wasm adapter
    // just calls `.map_err(to_js_error)`, so testing the inner crate is
    // sufficient to verify correctness.

    #[test]
    fn invalid_url_returns_error() {
        let result = url_parser::Url::parse("not-a-url");
        assert!(result.is_err());
    }

    #[test]
    fn invalid_percent_decode() {
        let result = url_parser::percent_decode("%GG");
        assert!(result.is_err());
    }
}
