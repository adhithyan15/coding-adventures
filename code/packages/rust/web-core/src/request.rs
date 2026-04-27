//! Enriched HTTP request type.
//!
//! `WebRequest` wraps the raw `HttpRequest` from `embeddable-http-server` and
//! adds pre-parsed fields that handlers always need: route parameters extracted
//! by the `Router` and query string parameters parsed from the request target.

use std::collections::HashMap;
use std::net::SocketAddr;

use embeddable_http_server::HttpRequest;

/// An HTTP request enriched with routing context and parsed query parameters.
///
/// Constructed by `WebApp::handle` after the router matches a route. Handlers
/// and hooks receive a shared reference and must not modify the request.
#[derive(Debug, Clone)]
pub struct WebRequest {
    /// The raw HTTP request from the transport layer.
    pub http: HttpRequest,

    /// Named route parameters extracted by the router.
    ///
    /// For pattern `/hello/:name` matched against `/hello/Adhithya`, this map
    /// contains `{"name" => "Adhithya"}`. Empty for requests that bypassed
    /// routing (e.g. those short-circuited by a `before_routing` hook).
    pub route_params: HashMap<String, String>,

    /// Parsed query string parameters.
    ///
    /// For target `/search?q=rust&limit=10`, this map contains
    /// `{"q" => "rust", "limit" => "10"}`. Percent-encoding is decoded.
    pub query_params: HashMap<String, String>,

    /// The path component of the request target (no query string).
    path: String,
}

impl WebRequest {
    /// Build a `WebRequest` from a raw `HttpRequest`.
    ///
    /// `route_params` and `path` are supplied by the router after matching.
    /// `query_params` is parsed from the request target.
    pub(crate) fn new(
        http: HttpRequest,
        path: String,
        route_params: HashMap<String, String>,
        query_params: HashMap<String, String>,
    ) -> Self {
        Self {
            http,
            route_params,
            query_params,
            path,
        }
    }

    /// HTTP method, e.g. `"GET"`, `"POST"`.
    pub fn method(&self) -> &str {
        self.http.method()
    }

    /// Request path without the query string, e.g. `/hello/Adhithya`.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// First matching header value, ASCII case-insensitive.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.http.header(name)
    }

    /// Request body bytes.
    pub fn body(&self) -> &[u8] {
        &self.http.body
    }

    /// Parsed `Content-Type` media type, e.g. `"application/json"`.
    ///
    /// Returns `None` if the header is absent or unparseable.
    pub fn content_type(&self) -> Option<&str> {
        self.http.header("Content-Type")
    }

    /// Parsed `Content-Length` value.
    ///
    /// Returns `None` if the header is absent or not a valid integer.
    pub fn content_length(&self) -> Option<usize> {
        self.http.head.content_length()
    }

    /// Peer socket address of the TCP connection.
    pub fn peer_addr(&self) -> SocketAddr {
        self.http.connection.peer_addr
    }
}
