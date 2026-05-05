//! # conduit
//!
//! Rust-native Conduit web framework facade over `web-core`.
//!
//! This crate is the Rust sibling of the Ruby, Python, Lua, TypeScript, and
//! Elixir Conduit ports. It keeps the same small web-framework surface while
//! letting Rust users stay entirely in Rust:
//!
//! ```rust,no_run
//! use conduit::{html, Application, Server};
//!
//! let mut app = Application::new();
//! app.get("/", |_| html("<h1>Hello from Conduit!</h1>"));
//!
//! let mut server = Server::bind("127.0.0.1", 3000, app).unwrap();
//! server.serve().unwrap();
//! ```

use std::borrow::Cow;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::str::Utf8Error;
use std::sync::Arc;

use embeddable_http_server::{HttpRequest, HttpResponse, HttpServerOptions};
use tcp_runtime::{PlatformError, StopHandle};
use web_core::{LogLevel, WebApp, WebRequest, WebResponse};

/// Request type exposed to Rust Conduit handlers.
pub type Request = WebRequest;

/// Response type returned by Rust Conduit handlers.
pub type Response = WebResponse;

/// Handler function accepted by routes.
pub type Handler = Arc<dyn Fn(&Request) -> Response + Send + Sync + 'static>;

/// Registered route metadata for inspection and docs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteInfo {
    pub method: String,
    pub pattern: String,
}

/// Sinatra-style application facade over `web_core::WebApp`.
pub struct Application {
    inner: WebApp,
    settings: HashMap<String, String>,
    routes: Vec<RouteInfo>,
}

impl Application {
    /// Create an empty Conduit application.
    pub fn new() -> Self {
        Self {
            inner: WebApp::new(),
            settings: HashMap::new(),
            routes: Vec::new(),
        }
    }

    /// Store an application setting.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.settings.insert(key.into(), value.into());
    }

    /// Read an application setting.
    pub fn setting(&self, key: &str) -> Option<&str> {
        self.settings.get(key).map(String::as_str)
    }

    /// All application settings.
    pub fn settings(&self) -> &HashMap<String, String> {
        &self.settings
    }

    /// Registered routes in insertion order.
    pub fn routes(&self) -> &[RouteInfo] {
        &self.routes
    }

    /// Register a handler for an arbitrary HTTP method.
    pub fn route(
        &mut self,
        method: impl Into<String>,
        pattern: &str,
        handler: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) {
        let method = method.into().to_ascii_uppercase();
        self.routes.push(RouteInfo {
            method: method.clone(),
            pattern: pattern.to_string(),
        });
        self.inner.add(method, pattern, handler);
    }

    pub fn get(
        &mut self,
        pattern: &str,
        handler: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) {
        self.route("GET", pattern, handler);
    }

    pub fn post(
        &mut self,
        pattern: &str,
        handler: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) {
        self.route("POST", pattern, handler);
    }

    pub fn put(
        &mut self,
        pattern: &str,
        handler: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) {
        self.route("PUT", pattern, handler);
    }

    pub fn delete(
        &mut self,
        pattern: &str,
        handler: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) {
        self.route("DELETE", pattern, handler);
    }

    pub fn patch(
        &mut self,
        pattern: &str,
        handler: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) {
        self.route("PATCH", pattern, handler);
    }

    /// Register a before filter.
    ///
    /// Return `Some(response)` to short-circuit the request before route lookup,
    /// or `None` to continue.
    pub fn before(&mut self, hook: impl Fn(&Request) -> Option<Response> + Send + Sync + 'static) {
        self.inner.before_routing(hook);
    }

    /// Register an after filter.
    ///
    /// The filter observes every response and cannot modify it. Use
    /// `after_response` when response transformation is desired.
    pub fn after(&mut self, hook: impl Fn(&Request, &Response) + Send + Sync + 'static) {
        self.inner.after_handler(move |req, res| {
            hook(req, &res);
            res
        });
    }

    /// Register a response-transforming after hook.
    pub fn after_response(
        &mut self,
        hook: impl Fn(&Request, Response) -> Response + Send + Sync + 'static,
    ) {
        self.inner.after_handler(hook);
    }

    /// Register a custom not-found handler.
    pub fn not_found(&mut self, hook: impl Fn(&Request) -> Response + Send + Sync + 'static) {
        self.inner.on_not_found(hook);
    }

    /// Register a custom method-not-allowed handler.
    pub fn method_not_allowed(
        &mut self,
        hook: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) {
        self.inner.on_method_not_allowed(hook);
    }

    /// Register a custom handler-panic recovery hook.
    pub fn on_error(&mut self, hook: impl Fn(&Request, &str) -> Response + Send + Sync + 'static) {
        self.inner.on_handler_error(hook);
    }

    /// Register a structured log hook.
    pub fn on_log(
        &mut self,
        hook: impl Fn(LogLevel, &str, &HashMap<String, String>) + Send + Sync + 'static,
    ) {
        self.inner.on_log(hook);
    }

    /// Handle a raw HTTP request. Useful for tests and embedded runtimes.
    pub fn handle(&self, request: HttpRequest) -> HttpResponse {
        self.inner.handle(request)
    }

    /// Consume the facade and return the underlying `web-core` app.
    pub fn into_web_app(self) -> WebApp {
        self.inner
    }
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension helpers for Conduit requests.
pub trait RequestExt {
    fn param(&self, name: &str) -> Option<&str>;
    fn query(&self, name: &str) -> Option<&str>;
    fn body_text(&self) -> Result<&str, Utf8Error>;
    fn body_text_lossy(&self) -> Cow<'_, str>;
}

impl RequestExt for Request {
    fn param(&self, name: &str) -> Option<&str> {
        self.route_params.get(name).map(String::as_str)
    }

    fn query(&self, name: &str) -> Option<&str> {
        self.query_params.get(name).map(String::as_str)
    }

    fn body_text(&self) -> Result<&str, Utf8Error> {
        std::str::from_utf8(self.body())
    }

    fn body_text_lossy(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(self.body())
    }
}

/// Build a plain-text response.
pub fn text(body: impl Into<String>) -> Response {
    WebResponse::text(body)
}

/// Build a plain-text response with an explicit status.
pub fn text_status(status: u16, body: impl Into<String>) -> Response {
    WebResponse::new(status, body.into().into_bytes())
        .with_content_type("text/plain; charset=utf-8")
}

/// Build an HTML response.
pub fn html(body: impl Into<String>) -> Response {
    html_status(200, body)
}

/// Build an HTML response with an explicit status.
pub fn html_status(status: u16, body: impl Into<String>) -> Response {
    WebResponse::new(status, body.into().into_bytes()).with_content_type("text/html; charset=utf-8")
}

/// Build a JSON response from pre-serialized JSON bytes or text.
pub fn json(body: impl AsRef<[u8]>) -> Response {
    WebResponse::json(body.as_ref().to_vec())
}

/// Build a JSON response with an explicit status.
pub fn json_status(status: u16, body: impl AsRef<[u8]>) -> Response {
    WebResponse::new(status, body.as_ref().to_vec()).with_content_type("application/json")
}

/// Build a redirect response.
pub fn redirect(location: impl Into<String>, status: u16) -> Response {
    WebResponse::new(status, Vec::new()).with_header("location", location)
}

/// Build an early-return response.
///
/// In Rust this is just a response value, so route handlers can return it and
/// before filters can wrap it in `Some(...)`.
pub fn halt(status: u16, body: impl Into<String>) -> Response {
    text_status(status, body)
}

/// Escape a string for inclusion in hand-built JSON.
///
/// Conduit intentionally avoids taking a JSON serializer dependency in this
/// facade. Higher-level apps can use their preferred serializer and pass the
/// resulting bytes to `json`.
pub fn escape_json_string(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if c <= '\u{1f}' => {
                out.push_str("\\u");
                out.push_str(&format!("{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

#[cfg(target_os = "linux")]
type NativeServer = web_core::WebServer<transport_platform::linux::EpollTransportPlatform>;

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
type NativeServer = web_core::WebServer<transport_platform::bsd::KqueueTransportPlatform>;

#[cfg(target_os = "windows")]
type NativeServer = web_core::WebServer<transport_platform::windows::WindowsTransportPlatform>;

/// Platform-native HTTP server for a Conduit application.
pub struct Server {
    inner: NativeServer,
}

impl Server {
    /// Bind to `host:port` with default HTTP options.
    pub fn bind(host: &str, port: u16, app: Application) -> Result<Self, PlatformError> {
        Self::bind_with_options(host, port, HttpServerOptions::default(), app)
    }

    /// Bind to `host:port` with explicit HTTP options.
    pub fn bind_with_options(
        host: &str,
        port: u16,
        options: HttpServerOptions,
        app: Application,
    ) -> Result<Self, PlatformError> {
        let addr = format!("{host}:{port}");
        let app = Arc::new(app.into_web_app());

        #[cfg(target_os = "linux")]
        let inner = web_core::WebServer::bind_epoll(addr, options, app)?;

        #[cfg(any(
            target_os = "macos",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            target_os = "dragonfly"
        ))]
        let inner = web_core::WebServer::bind_kqueue(addr, options, app)?;

        #[cfg(target_os = "windows")]
        let inner = web_core::WebServer::bind_windows(addr, options, app)?;

        Ok(Self { inner })
    }

    /// The local socket address the server bound to.
    pub fn local_addr(&self) -> SocketAddr {
        self.inner.local_addr()
    }

    /// Handle that can stop the server from another thread.
    pub fn stop_handle(&self) -> StopHandle {
        self.inner.stop_handle()
    }

    /// Serve requests until stopped.
    pub fn serve(&mut self) -> Result<(), PlatformError> {
        self.inner.serve()
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::{SocketAddr, TcpStream};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    use embeddable_http_server::HttpRequest;
    use http_core::{Header, HttpVersion, RequestHead};
    use tcp_runtime::{ConnectionId, TcpConnectionInfo};

    use super::*;

    fn make_request(method: &str, target: &str, body: &str) -> HttpRequest {
        HttpRequest {
            connection: TcpConnectionInfo {
                id: ConnectionId(0),
                peer_addr: SocketAddr::from(([127, 0, 0, 1], 1024)),
                local_addr: SocketAddr::from(([127, 0, 0, 1], 3000)),
            },
            head: RequestHead {
                method: method.to_string(),
                target: target.to_string(),
                version: HttpVersion { major: 1, minor: 1 },
                headers: vec![
                    Header {
                        name: "Host".into(),
                        value: "localhost".into(),
                    },
                    Header {
                        name: "Content-Length".into(),
                        value: body.len().to_string(),
                    },
                ],
            },
            body: body.as_bytes().to_vec(),
        }
    }

    fn request(port: u16, method: &str, path: &str, body: &str) -> (u16, String) {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();

        let req = format!(
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(req.as_bytes()).expect("write request");

        let mut reader = BufReader::new(&stream);
        let mut status_line = String::new();
        reader.read_line(&mut status_line).expect("read status");
        let status = status_line
            .split_whitespace()
            .nth(1)
            .expect("status code")
            .parse()
            .expect("parse status");

        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("read header");
            let trimmed = line.trim();
            if trimmed.is_empty() {
                break;
            }
            if trimmed.to_ascii_lowercase().starts_with("content-length:") {
                content_length = trimmed
                    .split_once(':')
                    .map(|(_, value)| value.trim().parse().unwrap_or(0))
                    .unwrap_or(0);
            }
        }

        let mut body_buf = vec![0; content_length];
        reader.read_exact(&mut body_buf).unwrap_or(());
        (status, String::from_utf8_lossy(&body_buf).into_owned())
    }

    #[test]
    fn response_helpers_set_expected_status_and_content_type() {
        let res = html_status(201, "<h1>created</h1>");
        assert_eq!(res.status, 201);
        assert_eq!(
            res.headers,
            vec![("content-type".into(), "text/html; charset=utf-8".into())]
        );

        let res = redirect("/", 301);
        assert_eq!(res.status, 301);
        assert_eq!(res.headers, vec![("location".into(), "/".into())]);
    }

    #[test]
    fn application_routes_named_params_and_settings() {
        let mut app = Application::new();
        app.set("app_name", "Conduit");
        assert_eq!(app.setting("app_name"), Some("Conduit"));

        app.get("/hello/:name", |req| {
            text(format!("Hello {}", req.param("name").unwrap_or("world")))
        });

        assert_eq!(
            app.routes(),
            &[RouteInfo {
                method: "GET".into(),
                pattern: "/hello/:name".into()
            }]
        );

        let res = app.handle(make_request("GET", "/hello/Adhithya", ""));
        assert_eq!(res.status, 200);
        assert_eq!(res.body, b"Hello Adhithya");
    }

    #[test]
    fn before_after_not_found_and_error_hooks_work() {
        let after_count = Arc::new(AtomicUsize::new(0));
        let after_seen = Arc::clone(&after_count);

        let mut app = Application::new();
        app.before(|req| {
            if req.path() == "/down" {
                Some(halt(503, "maintenance"))
            } else {
                None
            }
        });
        app.after(move |_req, _res| {
            after_seen.fetch_add(1, Ordering::SeqCst);
        });
        app.not_found(|req| html_status(404, format!("missing {}", req.path())));
        app.on_error(|_req, msg| json_status(500, format!(r#"{{"error":"{}"}}"#, msg)));
        app.get("/", |_| text("ok"));
        app.get("/boom", |_| panic!("intentional panic"));

        let res = app.handle(make_request("GET", "/down", ""));
        assert_eq!(res.status, 503);
        assert_eq!(res.body, b"maintenance");

        let res = app.handle(make_request("GET", "/missing", ""));
        assert_eq!(res.status, 404);
        assert_eq!(res.body, b"missing /missing");

        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let res = app.handle(make_request("GET", "/boom", ""));
        std::panic::set_hook(previous_hook);
        assert_eq!(res.status, 500);
        assert_eq!(res.body, br#"{"error":"intentional panic"}"#);

        assert_eq!(after_count.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn request_extension_reads_query_and_body() {
        let mut app = Application::new();
        app.post("/echo", |req| {
            text(format!(
                "{}:{}",
                req.query("name").unwrap_or("unknown"),
                req.body_text().unwrap_or("")
            ))
        });

        let res = app.handle(make_request("POST", "/echo?name=Conduit", "hello"));
        assert_eq!(res.status, 200);
        assert_eq!(res.body, b"Conduit:hello");
    }

    #[test]
    fn server_binds_and_serves_requests() {
        let mut app = Application::new();
        app.get("/hello/:name", |req| {
            text(format!("Hello {}", req.param("name").unwrap_or("world")))
        });

        let mut server = Server::bind("127.0.0.1", 0, app).expect("bind server");
        let port = server.local_addr().port();
        let stop = server.stop_handle();

        let handle = thread::spawn(move || {
            let _ = server.serve();
        });
        thread::sleep(Duration::from_millis(20));

        let (status, body) = request(port, "GET", "/hello/Rust", "");
        assert_eq!(status, 200);
        assert_eq!(body, "Hello Rust");

        stop.stop();
        handle.join().expect("server thread joins");
    }

    #[test]
    fn json_string_escaping_handles_control_characters() {
        assert_eq!(escape_json_string("a\"b\\c\n"), "a\\\"b\\\\c\\n");
    }
}
