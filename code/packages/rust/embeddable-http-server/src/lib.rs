//! Embeddable HTTP/1 server primitive built on `tcp-runtime`.
//!
//! The TCP runtime owns sockets and native readiness. This crate owns HTTP/1
//! request framing and response serialization, then hands complete requests to
//! an application callback. Language bridges can later expose the callback as a
//! Rack-like entry point without making the lower TCP runtime HTTP-aware.

use std::fmt;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;

use embeddable_tcp_server::{EmbeddableTcpServer, EmbeddableTcpServerOptions, TcpMailboxFrame};
use generic_job_protocol::{JobRequest, JobResult};
use http1::{parse_request_head, Http1ParseError};
use http_core::{BodyKind, Header, RequestHead};
use tcp_runtime::{
    PlatformError, ShardedStopHandle, ShardedTcpRuntime, TcpConnectionInfo, TcpHandlerResult,
    TcpRuntime, TcpRuntimeOptions,
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

/// A parallel HTTP/1 server: the **sharded** counterpart of [`HttpServer`]
/// (LANG-FULL / WEB01a-1).
///
/// `HttpServer` drives every connection on a single reactor thread, so handlers
/// never overlap — one slow request stalls all others. `ShardedHttpServer` runs
/// the **same** per-connection [`HttpConnectionState`] machine across
/// `worker_count` reactor threads (a [`ShardedTcpRuntime`]). Connections are
/// distributed across the shards (with an explicit accept fan-out on macOS/BSD,
/// since `SO_REUSEPORT` does not load-balance accepts there), so requests on
/// *different* connections handled by different shards run **concurrently**.
///
/// The request handler contract is unchanged: it is still a synchronous
/// `Fn(HttpRequest) -> HttpResponse` invoked inline on the owning shard, so the
/// response is produced and written on the same thread and HTTP/1.1 response
/// ordering on a single connection is preserved automatically. The handler is
/// shared (`Arc`) across all shards and must therefore be `Send + Sync`.
///
/// Two pipelined requests on the *same* connection still serialise — that is
/// correct for HTTP/1.1 (responses must be written in request order). Parallelism
/// is across connections, bounded by `worker_count`.
pub struct ShardedHttpServer<P> {
    runtime: ShardedTcpRuntime<P, HttpConnectionState>,
}

impl<P> ShardedHttpServer<P> {
    /// The local socket address the server is bound to.
    pub fn local_addr(&self) -> SocketAddr {
        self.runtime.local_addr()
    }

    /// The number of reactor shards (handler-parallelism degree).
    pub fn worker_count(&self) -> usize {
        self.runtime.worker_count()
    }

    /// A handle that can stop every shard from another thread.
    pub fn stop_handle(&self) -> ShardedStopHandle {
        self.runtime.stop_handle()
    }
}

impl<P> ShardedHttpServer<P>
where
    P: transport_platform::TransportPlatform + Send + 'static,
{
    /// Run all shard reactors until stopped. Blocks the calling thread.
    pub fn serve(&mut self) -> Result<(), PlatformError> {
        self.runtime.serve()
    }
}

/// Build the `(init, receive)` closures shared by every platform's sharded bind.
/// The `on_close` is a no-op (mirrors [`HttpServer::bind`]). Factored out so the
/// per-platform constructors below stay one line of real logic each.
///
/// Returns the handler wrapped in an `Arc` plus a clone of the options for the
/// per-connection state factory — both captured by the returned closures.
macro_rules! sharded_http_closures {
    ($handler:expr, $options:expr) => {{
        let handler: HttpHandler = Arc::new($handler);
        let state_options = $options.clone();
        (
            move |_info: TcpConnectionInfo| HttpConnectionState::new(&state_options),
            move |info: TcpConnectionInfo, state: &mut HttpConnectionState, bytes: &[u8]| {
                state.receive(info, bytes, &handler)
            },
        )
    }};
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
impl ShardedHttpServer<transport_platform::bsd::KqueueTransportPlatform> {
    /// Bind a kqueue-backed sharded server (macOS / BSD) with `worker_count`
    /// reactor shards.
    pub fn bind_kqueue_sharded<A, F>(
        addr: A,
        options: HttpServerOptions,
        worker_count: usize,
        handler: F,
    ) -> Result<Self, PlatformError>
    where
        A: ToSocketAddrs,
        F: Fn(HttpRequest) -> HttpResponse + Send + Sync + 'static,
    {
        let (init, receive) = sharded_http_closures!(handler, options);
        let runtime = TcpRuntime::bind_kqueue_sharded_with_state(
            addr,
            options.tcp,
            worker_count,
            init,
            receive,
            |_, _| {},
        )?;
        Ok(Self { runtime })
    }
}

#[cfg(target_os = "linux")]
impl ShardedHttpServer<transport_platform::linux::EpollTransportPlatform> {
    /// Bind an epoll-backed sharded server (Linux) with `worker_count` reactor
    /// shards.
    pub fn bind_epoll_sharded<A, F>(
        addr: A,
        options: HttpServerOptions,
        worker_count: usize,
        handler: F,
    ) -> Result<Self, PlatformError>
    where
        A: ToSocketAddrs,
        F: Fn(HttpRequest) -> HttpResponse + Send + Sync + 'static,
    {
        let (init, receive) = sharded_http_closures!(handler, options);
        let runtime = TcpRuntime::bind_epoll_sharded_with_state(
            addr,
            options.tcp,
            worker_count,
            init,
            receive,
            |_, _| {},
        )?;
        Ok(Self { runtime })
    }
}

#[cfg(target_os = "windows")]
impl ShardedHttpServer<transport_platform::windows::WindowsTransportPlatform> {
    /// Bind an IOCP-backed sharded server (Windows) with `worker_count` reactor
    /// shards.
    pub fn bind_windows_sharded<A, F>(
        addr: A,
        options: HttpServerOptions,
        worker_count: usize,
        handler: F,
    ) -> Result<Self, PlatformError>
    where
        A: ToSocketAddrs,
        F: Fn(HttpRequest) -> HttpResponse + Send + Sync + 'static,
    {
        let (init, receive) = sharded_http_closures!(handler, options);
        let runtime = TcpRuntime::bind_windows_sharded_with_state(
            addr,
            options.tcp,
            worker_count,
            init,
            receive,
            |_, _| {},
        )?;
        Ok(Self { runtime })
    }
}

/// A **deferred-response / mailbox** HTTP server (WEB01b-1a): per-request
/// parallelism via an in-process worker pool.
///
/// `HttpServer` runs one reactor with an inline handler; `ShardedHttpServer`
/// runs N reactors with inline handlers (parallel *by connection*).
/// `MailboxHttpServer` runs a single reactor that, on framing a complete
/// request, **submits it as a job** to a `worker_count`-thread pool and returns
/// immediately; a worker runs the handler and a response-router thread writes the
/// serialized response back to the originating connection. Handler concurrency is
/// thereby decoupled from the I/O thread — parallel *by request*.
///
/// Each framed request is submitted to the pool as it arrives, so a single
/// connection can have many requests in flight at once (full HTTP/1.1
/// pipelining). **WEB01b-1b** keeps the wire correct: the server enables the
/// mailbox's `ordered_responses`, so the response router writes each connection's
/// replies in **submission order** (a per-connection reorder buffer) even though
/// the worker pool finishes them out of order. The reorder buffer is bounded by
/// the pool's queue depth — a connection that pipelines past it is shed with a
/// 503 (backpressure), never unbounded buffering. (We intentionally do not use
/// `stream-reactor`'s `defer_read` for gating: it replays the deferred chunk on
/// resume, which would corrupt framing for bytes the handler already consumed —
/// see the handler comment in `bind`.) Parallelism is across requests, bounded by
/// `worker_count`. The platform (kqueue / epoll / IOCP) is selected internally by
/// `EmbeddableTcpServer`, so this type is cross-platform with no per-OS binds.
#[derive(Clone)]
pub struct MailboxHttpServer {
    inner: EmbeddableTcpServer<HttpConnectionState>,
}

impl MailboxHttpServer {
    /// Bind `host:port` with a `worker_count`-thread handler pool.
    pub fn bind<F>(
        host: &str,
        port: u16,
        options: HttpServerOptions,
        worker_count: usize,
        handler: F,
    ) -> std::io::Result<Self>
    where
        F: Fn(HttpRequest) -> HttpResponse + Send + Sync + 'static,
    {
        let handler = Arc::new(handler);
        let state_options = options.clone();
        let inner = EmbeddableTcpServer::new_inprocess_mailbox(
            EmbeddableTcpServerOptions {
                host: host.to_string(),
                port,
                worker_processes: worker_count.max(1),
                // WEB01b-1b: write each connection's responses back in submission
                // order so a pipelined keep-alive connection's replies stay in
                // HTTP/1.1 request order even when the pool finishes them out of
                // order (the pool is unordered). The reorder buffer is bounded by
                // the pool's queue depth — a connection that pipelines past it gets
                // a 503 (backpressure), not unbounded buffering.
                ordered_responses: true,
                ..EmbeddableTcpServerOptions::default()
            },
            // init — one HTTP connection state (buffer + limits) per connection.
            move |_info: TcpConnectionInfo| HttpConnectionState::new(&state_options),
            // handler — frame each complete request as it arrives and submit it as
            // a job to the pool. The worker runs the handler off the reactor thread
            // and the response router writes the reply back.
            //
            // Once the buffer holds no further complete request we return
            // `default()` (keep reading) — deliberately NOT `defer_read()`. In
            // `stream-reactor`, `defer_read` does not mean "pause output"; it means
            // "I did NOT consume these bytes — replay this chunk when reads resume."
            // Since we DID consume the bytes here (drained the buffer and submitted
            // the jobs), returning `defer_read` would have the reactor replay the
            // already-consumed chunk on the next `resume_all_reads()` (which fires
            // for ANY connection's response), re-feeding a possibly TCP-fragmented
            // tail into the buffer — corrupting framing (a duplicate submit, or a
            // malformed-head 400 emitted before the real response). So we keep
            // reading instead. Pipelined requests on one connection are all
            // submitted; the mailbox's `ordered_responses` (enabled below) writes
            // their replies back in submission order via a per-connection reorder
            // buffer (WEB01b-1b), so the wire stays HTTP/1.1-correct.
            |info: TcpConnectionInfo, state: &mut HttpConnectionState, bytes: &[u8], submitter| {
                state.buffer.extend_from_slice(bytes);
                // Drain EVERY complete request the read delivered — a single TCP
                // read can carry more than one (coalesced segments, or a pipelined
                // client). Calling `pop_request` only once would strand the extras
                // in the buffer until the next read, hanging a client that sent them
                // together and then waited. Each iteration submits one framed
                // request; `pop_request` returns `Ok(None)` when only a partial
                // request remains (keep reading) and `Err` on a malformed/oversize
                // one (close with an error response).
                loop {
                    match state.pop_request(info) {
                        Ok(Some(request)) => match submitter.submit(info.id, request) {
                            // Keep draining any further buffered requests.
                            Ok(_) => continue,
                            // Pool queue full → shed load with 503 (backpressure),
                            // rather than buffering unboundedly.
                            Err(_) => {
                                return TcpHandlerResult::write_and_close(serialize_response(
                                    &HttpResponse::new(503, "Service Unavailable")
                                        .with_header("Content-Type", "text/plain")
                                        .close(),
                                ))
                            }
                        },
                        Ok(None) => return TcpHandlerResult::default(),
                        Err(error) => {
                            return TcpHandlerResult::write_and_close(serialize_response(
                                &error_response(error),
                            ))
                        }
                    }
                }
            },
            // on_close — nothing connection-specific to release.
            |_info, _state| {},
            // map_response — serialize the worker's response back to the connection.
            |response: HttpResponse| {
                let close = response.close;
                let bytes = serialize_response(&response);
                Ok(if close {
                    TcpMailboxFrame::write_and_close(bytes)
                } else {
                    TcpMailboxFrame::write(bytes)
                })
            },
            // worker_fn — run the user handler on a pool thread; honor Connection: close.
            move |job: JobRequest<HttpRequest>| {
                let request = job.payload;
                let wants_close = request.wants_connection_close();
                let mut response = handler(request);
                response.close = response.close || wants_close;
                JobResult::Ok { payload: response }
            },
        )?;
        Ok(Self { inner })
    }

    /// The local socket address the server is bound to.
    pub fn local_addr(&self) -> SocketAddr {
        self.inner.local_addr()
    }

    /// Whether the server is currently serving.
    pub fn is_running(&self) -> bool {
        self.inner.is_running()
    }

    /// Signal the server to stop.
    pub fn stop(&self) {
        self.inner.stop();
    }

    /// Run the event loop until stopped. Blocks the calling thread.
    pub fn serve(&self) -> std::io::Result<()> {
        self.inner.serve()
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
    HttpResponse::new(status, message.as_bytes())
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
    use std::io::{self, Read, Write};
    use std::net::{Shutdown, TcpStream};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::Duration;

    const DEFAULT_STRESS_CLIENTS: usize = 128;
    const DEFAULT_STRESS_REQUESTS_PER_CLIENT: usize = 4;

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

    #[test]
    #[ignore]
    fn native_http_server_handles_pipelined_requests_under_concurrent_load() {
        let client_count = stress_count("EMBEDDABLE_HTTP_STRESS_CLIENTS", DEFAULT_STRESS_CLIENTS);
        let requests_per_client = stress_count(
            "EMBEDDABLE_HTTP_STRESS_REQUESTS_PER_CLIENT",
            DEFAULT_STRESS_REQUESTS_PER_CLIENT,
        );
        let expected_requests = client_count.saturating_mul(requests_per_client);
        let seen_requests = Arc::new(AtomicUsize::new(0));
        let handler_seen = Arc::clone(&seen_requests);
        let options = HttpServerOptions {
            tcp: TcpRuntimeOptions {
                max_connections: client_count.saturating_add(64),
                read_buffer_size: 2048,
                poll_timeout: Duration::from_millis(1),
                ..TcpRuntimeOptions::default()
            },
            ..HttpServerOptions::default()
        };
        let mut server = bind_native_http_server(("127.0.0.1", 0), options, move |request| {
            handler_seen.fetch_add(1, Ordering::SeqCst);
            HttpResponse::ok(format!("ok:{}:{}", request.method(), request.target()))
                .with_header("Content-Type", "text/plain")
        })
        .expect("bind native HTTP server");
        let addr = server.local_addr();
        let stop = server.stop_handle();
        let server_thread = thread::spawn(move || server.serve());
        let barrier = Arc::new(Barrier::new(client_count));

        let clients = (0..client_count)
            .map(|client_index| {
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    exercise_http_client(addr, client_index, requests_per_client)
                })
            })
            .collect::<Vec<_>>();

        for client in clients {
            client
                .join()
                .expect("HTTP stress client thread")
                .expect("HTTP stress client");
        }

        stop.stop();
        server_thread
            .join()
            .expect("HTTP server thread")
            .expect("HTTP server result");
        assert_eq!(seen_requests.load(Ordering::SeqCst), expected_requests);
    }

    fn stress_count(var: &str, default: usize) -> usize {
        std::env::var(var)
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(default)
    }

    fn exercise_http_client(
        addr: SocketAddr,
        client_index: usize,
        requests_per_client: usize,
    ) -> io::Result<()> {
        let mut stream = TcpStream::connect(addr)?;
        stream.set_nodelay(true)?;
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;

        let mut request_bytes = Vec::new();
        let mut expected_bodies = Vec::with_capacity(requests_per_client);
        for request_index in 0..requests_per_client {
            let target = format!("/stress/{client_index}/{request_index}");
            expected_bodies.push(format!("ok:GET:{target}"));
            request_bytes.extend_from_slice(
                format!(
                    "GET {target} HTTP/1.1\r\nHost: localhost\r\n{}\r\n",
                    if request_index + 1 == requests_per_client {
                        "Connection: close\r\n"
                    } else {
                        ""
                    }
                )
                .as_bytes(),
            );
        }

        stream.write_all(&request_bytes)?;
        stream.shutdown(Shutdown::Write)?;

        let mut response = Vec::new();
        stream.read_to_end(&mut response)?;
        let text = String::from_utf8(response).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("HTTP response was not UTF-8: {error}"),
            )
        })?;
        let response_count = text.matches("HTTP/1.1 200 OK\r\n").count();
        if response_count != requests_per_client {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("expected {requests_per_client} responses, got {response_count}: {text}"),
            ));
        }
        for body in expected_bodies {
            if !text.contains(&body) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("missing response body {body}: {text}"),
                ));
            }
        }
        Ok(())
    }

    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    fn bind_native_http_server<A, F>(
        addr: A,
        options: HttpServerOptions,
        handler: F,
    ) -> Result<HttpServer<transport_platform::bsd::KqueueTransportPlatform>, PlatformError>
    where
        A: ToSocketAddrs,
        F: Fn(HttpRequest) -> HttpResponse + Send + Sync + 'static,
    {
        HttpServer::bind_kqueue(addr, options, handler)
    }

    #[cfg(target_os = "linux")]
    fn bind_native_http_server<A, F>(
        addr: A,
        options: HttpServerOptions,
        handler: F,
    ) -> Result<HttpServer<transport_platform::linux::EpollTransportPlatform>, PlatformError>
    where
        A: ToSocketAddrs,
        F: Fn(HttpRequest) -> HttpResponse + Send + Sync + 'static,
    {
        HttpServer::bind_epoll(addr, options, handler)
    }

    #[cfg(target_os = "windows")]
    fn bind_native_http_server<A, F>(
        addr: A,
        options: HttpServerOptions,
        handler: F,
    ) -> Result<HttpServer<transport_platform::windows::WindowsTransportPlatform>, PlatformError>
    where
        A: ToSocketAddrs,
        F: Fn(HttpRequest) -> HttpResponse + Send + Sync + 'static,
    {
        HttpServer::bind_windows(addr, options, handler)
    }

    // ── WEB01a-1: sharded server bind + concurrency test ──────────────────────

    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    fn bind_native_sharded_http_server<A, F>(
        addr: A,
        options: HttpServerOptions,
        worker_count: usize,
        handler: F,
    ) -> Result<ShardedHttpServer<transport_platform::bsd::KqueueTransportPlatform>, PlatformError>
    where
        A: ToSocketAddrs,
        F: Fn(HttpRequest) -> HttpResponse + Send + Sync + 'static,
    {
        ShardedHttpServer::bind_kqueue_sharded(addr, options, worker_count, handler)
    }

    #[cfg(target_os = "linux")]
    fn bind_native_sharded_http_server<A, F>(
        addr: A,
        options: HttpServerOptions,
        worker_count: usize,
        handler: F,
    ) -> Result<ShardedHttpServer<transport_platform::linux::EpollTransportPlatform>, PlatformError>
    where
        A: ToSocketAddrs,
        F: Fn(HttpRequest) -> HttpResponse + Send + Sync + 'static,
    {
        ShardedHttpServer::bind_epoll_sharded(addr, options, worker_count, handler)
    }

    #[cfg(target_os = "windows")]
    fn bind_native_sharded_http_server<A, F>(
        addr: A,
        options: HttpServerOptions,
        worker_count: usize,
        handler: F,
    ) -> Result<ShardedHttpServer<transport_platform::windows::WindowsTransportPlatform>, PlatformError>
    where
        A: ToSocketAddrs,
        F: Fn(HttpRequest) -> HttpResponse + Send + Sync + 'static,
    {
        ShardedHttpServer::bind_windows_sharded(addr, options, worker_count, handler)
    }

    /// A `ShardedHttpServer` with several reactor shards serves many concurrent
    /// clients correctly: every request gets its matching response and the
    /// handler is invoked exactly once per request, regardless of which shard a
    /// connection lands on. This proves the sharded wiring (WEB01a-1) preserves
    /// the single-server request/response contract under cross-connection
    /// parallelism. (Throughput *scaling* is measured separately on a CPU-bound
    /// benchmark — see WEB01a-2; an echo handler is latency-bound and would not
    /// show speedup here.)
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn sharded_http_server_serves_concurrent_clients_across_shards() {
        let worker_count = 4;
        let client_count = 16;
        let requests_per_client = 4;
        let seen_requests = Arc::new(AtomicUsize::new(0));
        let handler_seen = Arc::clone(&seen_requests);

        let mut server = bind_native_sharded_http_server(
            ("127.0.0.1", 0),
            HttpServerOptions::default(),
            worker_count,
            move |request| {
                handler_seen.fetch_add(1, Ordering::SeqCst);
                HttpResponse::ok(format!("ok:{}:{}", request.method(), request.target()))
                    .with_header("Content-Type", "text/plain")
            },
        )
        .expect("bind sharded HTTP server");
        assert_eq!(server.worker_count(), worker_count, "all requested shards spawned");

        let addr = server.local_addr();
        let stop = server.stop_handle();
        let server_thread = thread::spawn(move || server.serve());
        let barrier = Arc::new(Barrier::new(client_count));

        let clients = (0..client_count)
            .map(|client_index| {
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait(); // release all clients at once → spread across shards
                    exercise_http_client(addr, client_index, requests_per_client)
                })
            })
            .collect::<Vec<_>>();

        for client in clients {
            client
                .join()
                .expect("sharded client thread")
                .expect("sharded client");
        }

        stop.stop();
        server_thread
            .join()
            .expect("sharded server thread")
            .expect("sharded server result");
        assert_eq!(
            seen_requests.load(Ordering::SeqCst),
            client_count * requests_per_client,
            "every request handled exactly once across all shards",
        );
    }

    /// A `MailboxHttpServer` handles requests **concurrently** on a SINGLE reactor
    /// by submitting them to its worker pool (WEB01b-1a). Proven deterministically
    /// (not by wall-clock): each handler bumps an in-flight gauge, holds briefly,
    /// and records the max simultaneous handlers. A single reactor calling the
    /// handler inline could never exceed 1; observing >= 2 proves the pool runs
    /// handlers in parallel while the lone reactor keeps accepting connections.
    /// One request per connection (`Connection: close`) — 1a's supported case;
    /// pipelined-response gating/ordering is WEB01b-1b.
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn mailbox_http_server_handles_requests_concurrently() {
        let worker_count = 4;
        let client_count = 8;
        let in_flight = Arc::new(AtomicUsize::new(0));
        let max_in_flight = Arc::new(AtomicUsize::new(0));
        let handler_in_flight = Arc::clone(&in_flight);
        let handler_max = Arc::clone(&max_in_flight);

        let server = MailboxHttpServer::bind(
            "127.0.0.1",
            0,
            HttpServerOptions::default(),
            worker_count,
            move |request| {
                let now = handler_in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                handler_max.fetch_max(now, Ordering::SeqCst);
                thread::sleep(Duration::from_millis(40));
                handler_in_flight.fetch_sub(1, Ordering::SeqCst);
                HttpResponse::ok(format!("ok:{}:{}", request.method(), request.target()))
                    .with_header("Content-Type", "text/plain")
            },
        )
        .expect("bind mailbox HTTP server");

        let addr = server.local_addr();
        let serve_handle = {
            let server = server.clone();
            thread::spawn(move || server.serve())
        };
        // Give the reactor a moment to start accepting.
        thread::sleep(Duration::from_millis(50));

        // A realistic single-request client: connect, then (after the barrier)
        // write one `Connection: close` request and read the response to EOF —
        // WITHOUT half-closing the write side first (a client that `shutdown`s
        // its write half before reading would race the deferred write against the
        // FIN-driven close; real HTTP clients keep the connection open until they
        // have read the response).
        let barrier = Arc::new(Barrier::new(client_count));
        let clients = (0..client_count)
            .map(|client_index| {
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || -> io::Result<()> {
                    let mut stream = TcpStream::connect(addr)?;
                    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
                    barrier.wait(); // release together so handlers overlap in the pool
                    write!(
                        stream,
                        "GET /work/{client_index} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
                    )?;
                    let mut response = Vec::new();
                    stream.read_to_end(&mut response)?; // server closes after the response
                    let text = String::from_utf8_lossy(&response);
                    if !text.starts_with("HTTP/1.1 200") {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("expected 200, got: {text}"),
                        ));
                    }
                    Ok(())
                })
            })
            .collect::<Vec<_>>();

        for client in clients {
            client
                .join()
                .expect("mailbox client thread")
                .expect("mailbox client");
        }

        let observed_max = max_in_flight.load(Ordering::SeqCst);
        server.stop();
        let _ = serve_handle.join();
        assert!(
            observed_max >= 2,
            "expected concurrent handler execution via the pool on a single reactor, but the \
             max observed in-flight handlers was {observed_max} (inline dispatch never exceeds 1)",
        );
    }

    /// WEB01b-1b: a `MailboxHttpServer` writes a **pipelined** connection's
    /// responses in REQUEST order, even though the (unordered) worker pool
    /// finishes them out of order. Determinism: the handler for request `k` sleeps
    /// `(n-1-k)*30ms`, so the LAST pipelined request finishes FIRST in the pool.
    /// Without the per-connection reorder buffer the client would read the bodies
    /// reversed; with it (WEB01b-1b's `ordered_responses`), they come back
    /// `r0, r1, …, r{n-1}` — the request order.
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn mailbox_http_server_preserves_pipelined_response_order() {
        // Split a raw byte stream of back-to-back HTTP/1 responses into the body
        // of each, in wire order, using each response's Content-Length.
        fn parse_http_bodies(raw: &[u8]) -> Vec<String> {
            fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
                haystack.windows(needle.len()).position(|w| w == needle)
            }
            let mut bodies = Vec::new();
            let mut pos = 0;
            while pos < raw.len() {
                let rest = &raw[pos..];
                let Some(boundary) = find(rest, b"\r\n\r\n") else {
                    break;
                };
                let head = String::from_utf8_lossy(&rest[..boundary]);
                let content_length = head
                    .lines()
                    .find_map(|line| {
                        line.trim()
                            .to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|value| value.trim().parse::<usize>().unwrap_or(0))
                    })
                    .unwrap_or(0);
                let body_start = pos + boundary + 4;
                let body_end = body_start + content_length;
                if body_end > raw.len() {
                    break;
                }
                bodies.push(String::from_utf8_lossy(&raw[body_start..body_end]).into_owned());
                pos = body_end;
            }
            bodies
        }

        let worker_count = 4;
        let n = 4usize;

        let server = MailboxHttpServer::bind(
            "127.0.0.1",
            0,
            HttpServerOptions::default(),
            worker_count,
            move |request| {
                // "/rK" → sleep longer for smaller K, so completion order is reversed.
                let k: usize = request
                    .target()
                    .trim_start_matches("/r")
                    .parse()
                    .unwrap_or(0);
                thread::sleep(Duration::from_millis(((n - 1 - k) * 30) as u64));
                HttpResponse::ok(format!("r{k}")).with_header("Content-Type", "text/plain")
            },
        )
        .expect("bind mailbox HTTP server");

        let addr = server.local_addr();
        let serve_handle = {
            let server = server.clone();
            thread::spawn(move || server.serve())
        };
        thread::sleep(Duration::from_millis(50));

        // One connection; all N requests pipelined in a single write (keep-alive,
        // the last carrying `Connection: close` so the server closes after the
        // final response and we can read to EOF).
        let mut stream = TcpStream::connect(addr).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let mut pipelined = String::new();
        for k in 0..n {
            let close = if k == n - 1 { "Connection: close\r\n" } else { "" };
            pipelined.push_str(&format!("GET /r{k} HTTP/1.1\r\nHost: localhost\r\n{close}\r\n"));
        }
        stream
            .write_all(pipelined.as_bytes())
            .expect("write pipelined requests");

        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).expect("read all responses");

        let bodies = parse_http_bodies(&raw);
        server.stop();
        let _ = serve_handle.join();

        let expected: Vec<String> = (0..n).map(|k| format!("r{k}")).collect();
        assert_eq!(
            bodies, expected,
            "pipelined responses must be written in request order despite out-of-order \
             completion (got {bodies:?})",
        );
    }
}
