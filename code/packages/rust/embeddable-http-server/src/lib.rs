//! Embeddable HTTP/1 server primitive built on `tcp-runtime`.
//!
//! The TCP runtime owns sockets and native readiness. This crate owns HTTP/1
//! request framing and response serialization, then hands complete requests to
//! an application callback. Language bridges can later expose the callback as a
//! Rack-like entry point without making the lower TCP runtime HTTP-aware.

use std::fmt;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;

use http1::{parse_request_head, Http1ParseError};
use http_core::{BodyKind, Header, RequestHead};
use tcp_runtime::{
    PlatformError, TcpConnectionInfo, TcpHandlerResult, TcpRuntime, TcpRuntimeOptions,
};

pub const VERSION: &str = "0.1.0";

const DEFAULT_MAX_REQUEST_HEAD_BYTES: usize = 16 * 1024;
const DEFAULT_MAX_REQUEST_BODY_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct HttpServerOptions {
    pub tcp: TcpRuntimeOptions,
    pub max_request_head_bytes: usize,
    pub max_request_body_bytes: usize,
}

impl Default for HttpServerOptions {
    fn default() -> Self {
        Self {
            tcp: TcpRuntimeOptions::default(),
            max_request_head_bytes: DEFAULT_MAX_REQUEST_HEAD_BYTES,
            max_request_body_bytes: DEFAULT_MAX_REQUEST_BODY_BYTES,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    pub connection: TcpConnectionInfo,
    pub head: RequestHead,
    pub body: Vec<u8>,
}

impl HttpRequest {
    pub fn method(&self) -> &str {
        &self.head.method
    }

    pub fn target(&self) -> &str {
        &self.head.target
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.head.header(name)
    }

    pub fn wants_connection_close(&self) -> bool {
        self.header("Connection")
            .map(|value| {
                value
                    .split(',')
                    .any(|part| part.trim().eq_ignore_ascii_case("close"))
            })
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub reason: String,
    pub headers: Vec<Header>,
    pub body: Vec<u8>,
    pub close: bool,
}

impl HttpResponse {
    pub fn new(status: u16, body: impl AsRef<[u8]>) -> Self {
        Self {
            status,
            reason: default_reason(status).to_string(),
            headers: Vec::new(),
            body: body.as_ref().to_vec(),
            close: false,
        }
    }

    pub fn ok(body: impl AsRef<[u8]>) -> Self {
        Self::new(200, body)
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push(Header {
            name: name.into(),
            value: value.into(),
        });
        self
    }

    pub fn close(mut self) -> Self {
        self.close = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpServerError {
    Parse(Http1ParseError),
    RequestHeadTooLarge,
    RequestBodyTooLarge,
    UnsupportedChunkedRequestBody,
    UnsupportedUntilEofRequestBody,
}

impl fmt::Display for HttpServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(f, "{error}"),
            Self::RequestHeadTooLarge => f.write_str("HTTP request head is too large"),
            Self::RequestBodyTooLarge => f.write_str("HTTP request body is too large"),
            Self::UnsupportedChunkedRequestBody => {
                f.write_str("chunked HTTP request bodies are not supported yet")
            }
            Self::UnsupportedUntilEofRequestBody => {
                f.write_str("EOF-delimited HTTP request bodies are not supported")
            }
        }
    }
}

impl std::error::Error for HttpServerError {}

impl From<Http1ParseError> for HttpServerError {
    fn from(value: Http1ParseError) -> Self {
        Self::Parse(value)
    }
}

pub type HttpHandler = Arc<dyn Fn(HttpRequest) -> HttpResponse + Send + Sync + 'static>;

#[derive(Debug, Clone)]
pub struct HttpConnectionState {
    buffer: Vec<u8>,
    limits: HttpServerLimits,
}

#[derive(Debug, Clone, Copy)]
struct HttpServerLimits {
    max_request_head_bytes: usize,
    max_request_body_bytes: usize,
}

impl HttpConnectionState {
    pub fn new(options: &HttpServerOptions) -> Self {
        Self {
            buffer: Vec::new(),
            limits: HttpServerLimits {
                max_request_head_bytes: options.max_request_head_bytes.max(1),
                max_request_body_bytes: options.max_request_body_bytes,
            },
        }
    }

    pub fn receive(
        &mut self,
        connection: TcpConnectionInfo,
        bytes: &[u8],
        handler: &HttpHandler,
    ) -> TcpHandlerResult {
        self.buffer.extend_from_slice(bytes);
        let mut output = Vec::new();
        let mut close = false;

        loop {
            match self.pop_request(connection) {
                Ok(Some(request)) => {
                    let request_close = request.wants_connection_close();
                    let mut response = handler(request);
                    response.close = response.close || request_close;
                    close = close || response.close;
                    output.extend(serialize_response(&response));
                    if close {
                        break;
                    }
                }
                Ok(None) => break,
                Err(error) => {
                    let response = error_response(error);
                    output.extend(serialize_response(&response));
                    close = true;
                    break;
                }
            }
        }

        if close {
            TcpHandlerResult::write_and_close(output)
        } else if output.is_empty() {
            TcpHandlerResult::default()
        } else {
            TcpHandlerResult::write(output)
        }
    }

    fn pop_request(
        &mut self,
        connection: TcpConnectionInfo,
    ) -> Result<Option<HttpRequest>, HttpServerError> {
        if self.buffer.len() > self.limits.max_request_head_bytes
            && !contains_head_terminator(&self.buffer)
        {
            return Err(HttpServerError::RequestHeadTooLarge);
        }

        let parsed = match parse_request_head(&self.buffer) {
            Ok(parsed) => parsed,
            Err(Http1ParseError::IncompleteHead) => return Ok(None),
            Err(error) => return Err(error.into()),
        };

        if parsed.body_offset > self.limits.max_request_head_bytes {
            return Err(HttpServerError::RequestHeadTooLarge);
        }

        let body_len = match parsed.body_kind {
            BodyKind::None => 0,
            BodyKind::ContentLength(length) => length,
            BodyKind::Chunked => return Err(HttpServerError::UnsupportedChunkedRequestBody),
            BodyKind::UntilEof => return Err(HttpServerError::UnsupportedUntilEofRequestBody),
        };
        if body_len > self.limits.max_request_body_bytes {
            return Err(HttpServerError::RequestBodyTooLarge);
        }

        let required = parsed.body_offset + body_len;
        if self.buffer.len() < required {
            return Ok(None);
        }

        let body = self.buffer[parsed.body_offset..required].to_vec();
        self.buffer.drain(..required);
        Ok(Some(HttpRequest {
            connection,
            head: parsed.head,
            body,
        }))
    }
}

pub struct HttpServer<P> {
    runtime: TcpRuntime<P, HttpConnectionState>,
}

impl<P> HttpServer<P>
where
    P: transport_platform::TransportPlatform,
{
    pub fn local_addr(&self) -> SocketAddr {
        self.runtime.local_addr()
    }

    pub fn stop_handle(&self) -> tcp_runtime::StopHandle {
        self.runtime.stop_handle()
    }

    pub fn serve(&mut self) -> Result<(), PlatformError> {
        self.runtime.serve()
    }
}

impl<P> HttpServer<P>
where
    P: transport_platform::TransportPlatform,
{
    pub fn bind<F>(
        platform: P,
        address: tcp_runtime::BindAddress,
        options: HttpServerOptions,
        handler: F,
    ) -> Result<Self, PlatformError>
    where
        F: Fn(HttpRequest) -> HttpResponse + Send + Sync + 'static,
    {
        let handler: HttpHandler = Arc::new(handler);
        let state_options = options.clone();
        let runtime = TcpRuntime::bind_with_state(
            platform,
            address,
            options.tcp,
            move |_| HttpConnectionState::new(&state_options),
            move |info, state, bytes| state.receive(info, bytes, &handler),
            |_, _| {},
        )?;
        Ok(Self { runtime })
    }
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
impl HttpServer<transport_platform::bsd::KqueueTransportPlatform> {
    pub fn bind_kqueue<A, F>(
        addr: A,
        options: HttpServerOptions,
        handler: F,
    ) -> Result<Self, PlatformError>
    where
        A: ToSocketAddrs,
        F: Fn(HttpRequest) -> HttpResponse + Send + Sync + 'static,
    {
        let address = resolve_first_socket_addr(addr)?;
        let platform = transport_platform::bsd::KqueueTransportPlatform::new()?;
        Self::bind(
            platform,
            tcp_runtime::BindAddress::Ip(address),
            options,
            handler,
        )
    }
}

#[cfg(target_os = "linux")]
impl HttpServer<transport_platform::linux::EpollTransportPlatform> {
    pub fn bind_epoll<A, F>(
        addr: A,
        options: HttpServerOptions,
        handler: F,
    ) -> Result<Self, PlatformError>
    where
        A: ToSocketAddrs,
        F: Fn(HttpRequest) -> HttpResponse + Send + Sync + 'static,
    {
        let address = resolve_first_socket_addr(addr)?;
        let platform = transport_platform::linux::EpollTransportPlatform::new()?;
        Self::bind(
            platform,
            tcp_runtime::BindAddress::Ip(address),
            options,
            handler,
        )
    }
}

#[cfg(target_os = "windows")]
impl HttpServer<transport_platform::windows::WindowsTransportPlatform> {
    pub fn bind_windows<A, F>(
        addr: A,
        options: HttpServerOptions,
        handler: F,
    ) -> Result<Self, PlatformError>
    where
        A: ToSocketAddrs,
        F: Fn(HttpRequest) -> HttpResponse + Send + Sync + 'static,
    {
        let address = resolve_first_socket_addr(addr)?;
        let platform = transport_platform::windows::WindowsTransportPlatform::new()?;
        Self::bind(
            platform,
            tcp_runtime::BindAddress::Ip(address),
            options,
            handler,
        )
    }
}

fn serialize_response(response: &HttpResponse) -> Vec<u8> {
    let mut output = Vec::new();
    let reason = if response.reason.is_empty() {
        default_reason(response.status)
    } else {
        &response.reason
    };
    output.extend_from_slice(format!("HTTP/1.1 {} {}\r\n", response.status, reason).as_bytes());

    let has_content_length = response
        .headers
        .iter()
        .any(|header| header.name.eq_ignore_ascii_case("Content-Length"));
    let has_connection = response
        .headers
        .iter()
        .any(|header| header.name.eq_ignore_ascii_case("Connection"));

    for header in &response.headers {
        output.extend_from_slice(header.name.as_bytes());
        output.extend_from_slice(b": ");
        output.extend_from_slice(header.value.as_bytes());
        output.extend_from_slice(b"\r\n");
    }
    if !has_content_length {
        output.extend_from_slice(format!("Content-Length: {}\r\n", response.body.len()).as_bytes());
    }
    if response.close && !has_connection {
        output.extend_from_slice(b"Connection: close\r\n");
    }

    output.extend_from_slice(b"\r\n");
    output.extend_from_slice(&response.body);
    output
}

fn error_response(error: HttpServerError) -> HttpResponse {
    let (status, message) = match error {
        HttpServerError::RequestHeadTooLarge | HttpServerError::RequestBodyTooLarge => {
            (413, "Payload Too Large")
        }
        HttpServerError::UnsupportedChunkedRequestBody
        | HttpServerError::UnsupportedUntilEofRequestBody => (501, "Not Implemented"),
        HttpServerError::Parse(_) => (400, "Bad Request"),
    };
    HttpResponse::new(status, message.as_bytes().to_vec())
        .with_header("Content-Type", "text/plain")
        .close()
}

fn contains_head_terminator(bytes: &[u8]) -> bool {
    bytes.windows(4).any(|window| window == b"\r\n\r\n")
        || bytes.windows(2).any(|window| window == b"\n\n")
}

fn default_reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        400 => "Bad Request",
        404 => "Not Found",
        413 => "Payload Too Large",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        _ => "OK",
    }
}

fn resolve_first_socket_addr<A: ToSocketAddrs>(addr: A) -> Result<SocketAddr, PlatformError> {
    addr.to_socket_addrs()
        .map_err(PlatformError::from)?
        .next()
        .ok_or_else(|| PlatformError::Io("no socket addresses resolved".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connection() -> TcpConnectionInfo {
        TcpConnectionInfo {
            id: tcp_runtime::ConnectionId(7),
            peer_addr: SocketAddr::from(([127, 0, 0, 1], 43_210)),
            local_addr: SocketAddr::from(([127, 0, 0, 1], 80)),
        }
    }

    #[test]
    fn serializes_simple_http_response() {
        let response = HttpResponse::ok("hello").with_header("Content-Type", "text/plain");
        let bytes = serialize_response(&response);
        let text = String::from_utf8(bytes).expect("response utf8");
        assert!(text.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(text.contains("Content-Length: 5\r\n"));
        assert!(text.ends_with("\r\n\r\nhello"));
    }

    #[test]
    fn buffers_fragmented_request_until_complete() {
        let mut state = HttpConnectionState::new(&HttpServerOptions::default());
        let handler: HttpHandler = Arc::new(|request| {
            assert_eq!(request.method(), "POST");
            assert_eq!(request.target(), "/submit");
            assert_eq!(request.body, b"hello");
            HttpResponse::ok("done")
        });

        let first = state.receive(
            connection(),
            b"POST /submit HTTP/1.1\r\nHost: example.com\r\nContent-Length: 5\r\n\r\nhe",
            &handler,
        );
        assert!(first.write.is_empty());
        assert!(!first.close);

        let second = state.receive(connection(), b"llo", &handler);
        let text = String::from_utf8(second.write).expect("response utf8");
        assert!(text.contains("Content-Length: 4\r\n"));
        assert!(text.ends_with("\r\n\r\ndone"));
    }

    #[test]
    fn handles_pipelined_requests_in_one_tcp_read() {
        let mut state = HttpConnectionState::new(&HttpServerOptions::default());
        let handler: HttpHandler =
            Arc::new(|request| HttpResponse::ok(format!("seen {}", request.target())));

        let result = state.receive(
            connection(),
            b"GET /one HTTP/1.1\r\n\r\nGET /two HTTP/1.1\r\n\r\n",
            &handler,
        );
        let text = String::from_utf8(result.write).expect("response utf8");
        assert!(text.contains("seen /one"));
        assert!(text.contains("seen /two"));
        assert!(!result.close);
    }

    #[test]
    fn parse_errors_close_connection_with_bad_request() {
        let mut state = HttpConnectionState::new(&HttpServerOptions::default());
        let handler: HttpHandler = Arc::new(|_| HttpResponse::ok("never"));

        let result = state.receive(connection(), b"bad\r\n\r\n", &handler);
        let text = String::from_utf8(result.write).expect("response utf8");
        assert!(text.starts_with("HTTP/1.1 400 Bad Request\r\n"));
        assert!(result.close);
    }

    #[test]
    fn chunked_requests_are_rejected_until_supported() {
        let mut state = HttpConnectionState::new(&HttpServerOptions::default());
        let handler: HttpHandler = Arc::new(|_| HttpResponse::ok("never"));

        let result = state.receive(
            connection(),
            b"POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n",
            &handler,
        );
        let text = String::from_utf8(result.write).expect("response utf8");
        assert!(text.starts_with("HTTP/1.1 501 Not Implemented\r\n"));
        assert!(result.close);
    }
}
