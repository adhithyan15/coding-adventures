//! HTTP response builder.
//!
//! `WebResponse` has a fluent builder API that is slightly richer than the raw
//! `HttpResponse` from `embeddable-http-server`. It converts losslessly into
//! `HttpResponse` for transport.

use embeddable_http_server::HttpResponse;
use http_core::Header;

/// An HTTP response produced by a handler or hook.
#[derive(Debug, Clone)]
pub struct WebResponse {
    /// HTTP status code, e.g. 200, 404, 500.
    pub status: u16,
    /// Response headers as `(name, value)` pairs.
    pub headers: Vec<(String, String)>,
    /// Response body bytes.
    pub body: Vec<u8>,
}

impl WebResponse {
    /// 200 OK with the given bytes as body.
    pub fn ok(body: impl Into<Vec<u8>>) -> Self {
        Self::new(200, body)
    }

    /// 200 OK with a `text/plain; charset=utf-8` body.
    pub fn text(body: impl Into<String>) -> Self {
        Self::ok(body.into().into_bytes()).with_content_type("text/plain; charset=utf-8")
    }

    /// 200 OK with an `application/json` body.
    pub fn json(body: impl Into<Vec<u8>>) -> Self {
        Self::ok(body).with_content_type("application/json")
    }

    /// 404 Not Found with a plain-text body.
    pub fn not_found() -> Self {
        Self::new(404, b"Not Found".to_vec()).with_content_type("text/plain")
    }

    /// 405 Method Not Allowed with a plain-text body.
    pub fn method_not_allowed() -> Self {
        Self::new(405, b"Method Not Allowed".to_vec()).with_content_type("text/plain")
    }

    /// 500 Internal Server Error with the given message as body.
    pub fn internal_error(message: impl AsRef<str>) -> Self {
        Self::new(500, message.as_ref().as_bytes().to_vec()).with_content_type("text/plain")
    }

    /// Arbitrary status code with the given bytes as body.
    pub fn new(status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: body.into(),
        }
    }

    /// Add a response header. Does not replace an existing header of the same
    /// name — call this once per unique header name.
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Set the `Content-Type` header.
    pub fn with_content_type(self, ct: impl Into<String>) -> Self {
        self.with_header("content-type", ct)
    }
}

impl From<WebResponse> for HttpResponse {
    fn from(web: WebResponse) -> Self {
        HttpResponse {
            status: web.status,
            reason: String::new(),
            headers: web
                .headers
                .into_iter()
                .map(|(name, value)| Header { name, value })
                .collect(),
            body: web.body,
            close: false,
        }
    }
}

impl From<HttpResponse> for WebResponse {
    fn from(http: HttpResponse) -> Self {
        Self {
            status: http.status,
            headers: http
                .headers
                .into_iter()
                .map(|h| (h.name, h.value))
                .collect(),
            body: http.body,
        }
    }
}
