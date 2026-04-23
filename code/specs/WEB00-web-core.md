# WEB00 — web-core

## Overview

`web-core` is a generic Rack/WSGI-like HTTP application layer built on top of
`embeddable-http-server`. It owns routing, request enrichment, response
building, and a lifecycle hook registry. Language packages — Ruby, Python, Lua,
Perl, and others — only need to implement their own application layer. All
shared concerns belong to `web-core` in Rust.

The design goal is that a Ruby developer writing a Sinatra-like DSL, a Python
developer writing a Flask-like DSL, or a Lua developer writing a lightweight
handler should all reach `web-core` as the single point of authority for:

- Path pattern matching with named parameters
- Request enrichment (query strings, route params, content type)
- Lifecycle hook dispatch (authentication, logging, error recovery)
- Response building with sane defaults

The TCP socket, HTTP/1 framing, and native platform event loop stay in the
layers below. The route handler logic and DSL idioms stay in the layers above.

```text
┌─────────────────────────────────────────────────────────┐
│  Language DSL layer                                       │
│  Ruby/conduit  Python/web  Lua/web  Perl/web  ...        │
│  Registers routes, hooks, and handler closures           │
└──────────────────────────┬──────────────────────────────┘
                           │  WebApp::handle(HttpRequest) → HttpResponse
┌──────────────────────────▼──────────────────────────────┐
│  web-core  (this crate)                                   │
│  WebRequest  WebResponse  Router  HookRegistry  WebApp   │
│  WebServer                                               │
└──────────────────────────┬──────────────────────────────┘
                           │  HttpHandler: Fn(HttpRequest) → HttpResponse
┌──────────────────────────▼──────────────────────────────┐
│  embeddable-http-server                                   │
│  HttpConnectionState  HttpServer                          │
└──────────────────────────┬──────────────────────────────┘
                           │  TcpHandlerResult
┌──────────────────────────▼──────────────────────────────┐
│  tcp-runtime + transport-platform                         │
│  native kqueue / epoll / IOCP event loop                  │
└─────────────────────────────────────────────────────────┘
```

## Why This Exists

`embeddable-http-server` provides raw `HttpRequest` and `HttpResponse` types and
calls a closure for every complete request. That is the right level of
abstraction for the transport layer, but it is too low for application code:

- Every language bridge independently reimplements path matching.
- Every bridge independently parses query strings.
- Every bridge independently applies routing parameters to request state.
- There is no shared lifecycle hook surface. Logging, authentication, and error
  recovery hooks are invented and discarded per bridge.

`web-core` moves these shared concerns into one Rust crate so that language
bridges stop duplicating them.

## Non-Goals

`web-core` is not:

- A replacement for the job queue / thread-pool mailbox pattern for CPU-bound
  or GVL-blocked work. That belongs to `embeddable-tcp-server` with its
  `new_inprocess_mailbox()` mode. `web-core` focuses on the synchronous request
  dispatch path first.
- A full-featured framework with sessions, cookies, templating, or asset
  serving. Those are language-layer concerns.
- An async framework. The HTTP/1 server is currently synchronous (one request
  at a time per connection). The async promotion path is described in
  `WEB01-async-web-core.md`.
- A multi-language runtime. `web-core` is a Rust crate. Language bridges
  produce `WebApp` values through safe Rust APIs and wire them into the server.

## Core Types

### WebRequest

`WebRequest` is the enriched request that handlers receive. It carries the raw
`HttpRequest` from `embeddable-http-server` plus pre-parsed fields that
handlers always need.

```rust
pub struct WebRequest {
    /// The raw HTTP request from the transport layer.
    pub http: HttpRequest,

    /// Named route parameters extracted by the router.
    ///
    /// For pattern `/hello/:name` matched against `/hello/Adhithya`,
    /// this map contains `{"name" => "Adhithya"}`.
    pub route_params: HashMap<String, String>,

    /// Parsed query string parameters.
    ///
    /// For target `/search?q=rust&limit=10`, this map contains
    /// `{"q" => "rust", "limit" => "10"}`.
    pub query_params: HashMap<String, String>,
}

impl WebRequest {
    /// HTTP method, e.g. `"GET"`, `"POST"`.
    pub fn method(&self) -> &str;

    /// Request path without the query string, e.g. `/hello/Adhithya`.
    pub fn path(&self) -> &str;

    /// First matching header value, ASCII case-insensitive.
    pub fn header(&self, name: &str) -> Option<&str>;

    /// Request body bytes.
    pub fn body(&self) -> &[u8];

    /// Parsed `Content-Type` media type, e.g. `"application/json"`.
    pub fn content_type(&self) -> Option<&str>;

    /// Parsed `Content-Length` in bytes.
    pub fn content_length(&self) -> Option<usize>;

    /// Peer socket address.
    pub fn peer_addr(&self) -> std::net::SocketAddr;
}
```

`WebRequest` is never constructed outside `web-core`. The router builds it from
`HttpRequest` after matching a route.

### WebResponse

`WebResponse` is the response returned by handlers and hooks. It is slightly
richer than `HttpResponse` in its builder API, but converts to `HttpResponse`
for transport.

```rust
pub struct WebResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl WebResponse {
    /// 200 OK with the given bytes as body.
    pub fn ok(body: impl Into<Vec<u8>>) -> Self;

    /// 200 OK with a `text/plain` body.
    pub fn text(body: impl Into<String>) -> Self;

    /// 200 OK with an `application/json` body.
    pub fn json(body: impl Into<Vec<u8>>) -> Self;

    /// 404 Not Found with a `text/plain` body.
    pub fn not_found() -> Self;

    /// 405 Method Not Allowed with a `text/plain` body.
    pub fn method_not_allowed() -> Self;

    /// 500 Internal Server Error with the given message.
    pub fn internal_error(message: impl AsRef<str>) -> Self;

    /// Arbitrary status code with the given bytes as body.
    pub fn new(status: u16, body: impl Into<Vec<u8>>) -> Self;

    /// Add or replace a response header.
    pub fn with_header(self, name: impl Into<String>, value: impl Into<String>) -> Self;

    /// Set the `Content-Type` header.
    pub fn with_content_type(self, ct: impl Into<String>) -> Self;
}

impl From<WebResponse> for HttpResponse { ... }
impl From<HttpResponse> for WebResponse { ... }
```

### Router

`Router` owns the route table. Routes are registered with an HTTP method, a
path pattern, and a handler closure. The pattern syntax reuses `RoutePattern`
from `http-core`.

```rust
/// A handler function: takes an enriched request and returns a response.
pub type Handler = Arc<dyn Fn(&WebRequest) -> WebResponse + Send + Sync + 'static>;

/// A matched route plus the extracted named parameters.
pub struct RouteMatch<'r> {
    pub route: &'r Route,
    pub params: Vec<(String, String)>,
}

pub struct Route {
    pub method: String,
    pub pattern: RoutePattern,
    pub handler: Handler,
}

pub struct Router {
    routes: Vec<Route>,
}

impl Router {
    pub fn new() -> Self;

    /// Register a handler for the given method and path pattern.
    pub fn add(
        &mut self,
        method: impl Into<String>,
        pattern: &str,
        handler: impl Fn(&WebRequest) -> WebResponse + Send + Sync + 'static,
    );

    /// Convenience wrapper for GET.
    pub fn get(&mut self, pattern: &str, handler: impl Fn(&WebRequest) -> WebResponse + Send + Sync + 'static);

    /// Convenience wrapper for POST.
    pub fn post(&mut self, pattern: &str, handler: impl Fn(&WebRequest) -> WebResponse + Send + Sync + 'static);

    /// Convenience wrapper for PUT.
    pub fn put(&mut self, pattern: &str, handler: impl Fn(&WebRequest) -> WebResponse + Send + Sync + 'static);

    /// Convenience wrapper for DELETE.
    pub fn delete(&mut self, pattern: &str, handler: impl Fn(&WebRequest) -> WebResponse + Send + Sync + 'static);

    /// Convenience wrapper for PATCH.
    pub fn patch(&mut self, pattern: &str, handler: impl Fn(&WebRequest) -> WebResponse + Send + Sync + 'static);

    /// Find the first route matching the given method and path.
    ///
    /// Returns `None` when no route matches.
    /// Returns the first match in registration order for overlapping patterns.
    pub fn match_request<'r>(&'r self, method: &str, path: &str) -> Option<RouteMatch<'r>>;
}
```

Route matching rules:

- Method comparison is ASCII case-insensitive.
- Path matching is exact for `Literal` segments and wildcard for `Param`
  segments. Query strings are stripped before matching.
- Routes are checked in registration order; the first match wins.
- A request with a path that matches a registered pattern but the wrong method
  returns a `405 Method Not Allowed` rather than a `404 Not Found`. The router
  surfaces this distinction through `RouteLookupResult`.

```rust
pub enum RouteLookupResult<'r> {
    Matched(RouteMatch<'r>),
    MethodNotAllowed,
    NotFound,
}

impl Router {
    pub fn lookup<'r>(&'r self, method: &str, path: &str) -> RouteLookupResult<'r>;
}
```

### HookRegistry

`HookRegistry` holds zero or more listeners for each lifecycle event. All hooks
are heap-allocated closures (`Arc<dyn Fn + Send + Sync>`), so they can be
registered from any thread and survive across request cycles.

Language bridges install hooks by wrapping their VM callbacks in these
closures. For GVL-constrained languages like Ruby, the closure body will call
`rb_thread_call_with_gvl` as the existing `conduit_native` dispatch already
does.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// A log entry produced by web-core or by application code.
pub struct LogEvent<'a> {
    pub level: LogLevel,
    pub message: &'a str,
    pub fields: &'a HashMap<String, String>,
}

pub struct HookRegistry {
    // Each field holds a Vec of registered listeners.
    // See registration methods below for types.
}

impl HookRegistry {
    pub fn new() -> Self;

    // --- Registration ---

    /// Called once after the server socket is bound and ready to accept.
    pub fn on_server_start(&mut self, hook: impl Fn(std::net::SocketAddr) + Send + Sync + 'static);

    /// Called once when the server event loop exits cleanly.
    pub fn on_server_stop(&mut self, hook: impl Fn() + Send + Sync + 'static);

    /// Called when a new TCP connection is accepted.
    pub fn on_connect(&mut self, hook: impl Fn(u64, std::net::SocketAddr) + Send + Sync + 'static);

    /// Called when a TCP connection closes (graceful or reset).
    pub fn on_disconnect(&mut self, hook: impl Fn(u64) + Send + Sync + 'static);

    /// Called before routing. Returning `Some(response)` short-circuits the
    /// rest of the pipeline and sends that response immediately.
    ///
    /// Use case: authentication, rate limiting, redirect.
    pub fn before_routing(&mut self, hook: impl Fn(&WebRequest) -> Option<WebResponse> + Send + Sync + 'static);

    /// Called when no route matches. The return value is the response sent to
    /// the client.
    ///
    /// Default: 404 Not Found.
    pub fn on_not_found(&mut self, hook: impl Fn(&WebRequest) -> WebResponse + Send + Sync + 'static);

    /// Called when a route matches but the method does not.
    ///
    /// Default: 405 Method Not Allowed.
    pub fn on_method_not_allowed(&mut self, hook: impl Fn(&WebRequest) -> WebResponse + Send + Sync + 'static);

    /// Called after routing but before the handler. Returning `Some(response)`
    /// short-circuits the handler.
    ///
    /// Use case: request validation, body size checks.
    pub fn before_handler(&mut self, hook: impl Fn(&WebRequest) -> Option<WebResponse> + Send + Sync + 'static);

    /// Called when a handler panics. The `error` string is the panic message
    /// (or a generic message if the panic value was not a string). The return
    /// value replaces the response.
    ///
    /// Default: 500 Internal Server Error.
    pub fn on_handler_error(&mut self, hook: impl Fn(&WebRequest, &str) -> WebResponse + Send + Sync + 'static);

    /// Called after the handler returns successfully. Each hook receives the
    /// current response and returns a (possibly modified) response.
    ///
    /// Use case: CORS headers, response compression, security headers.
    pub fn after_handler(&mut self, hook: impl Fn(&WebRequest, WebResponse) -> WebResponse + Send + Sync + 'static);

    /// Called after the response is handed to the transport layer. The third
    /// argument is the request duration in milliseconds.
    ///
    /// Hooks registered here must not block. Fire-and-forget is fine.
    /// Use case: access logging, metrics, tracing.
    pub fn after_send(&mut self, hook: impl Fn(&WebRequest, &WebResponse, u64) + Send + Sync + 'static);

    /// Structured log sink. web-core emits internal log events here.
    /// Application code can call `WebApp::log()` to emit through the same
    /// sink.
    pub fn on_log(&mut self, hook: impl Fn(LogLevel, &str, &HashMap<String, String>) + Send + Sync + 'static);

    // --- Execution (called by WebApp, not by application code) ---

    pub(crate) fn fire_server_start(&self, addr: std::net::SocketAddr);
    pub(crate) fn fire_server_stop(&self);
    pub(crate) fn fire_connect(&self, connection_id: u64, peer_addr: std::net::SocketAddr);
    pub(crate) fn fire_disconnect(&self, connection_id: u64);
    pub(crate) fn run_before_routing(&self, req: &WebRequest) -> Option<WebResponse>;
    pub(crate) fn run_on_not_found(&self, req: &WebRequest) -> WebResponse;
    pub(crate) fn run_on_method_not_allowed(&self, req: &WebRequest) -> WebResponse;
    pub(crate) fn run_before_handler(&self, req: &WebRequest) -> Option<WebResponse>;
    pub(crate) fn run_on_handler_error(&self, req: &WebRequest, error: &str) -> WebResponse;
    pub(crate) fn run_after_handler(&self, req: &WebRequest, response: WebResponse) -> WebResponse;
    pub(crate) fn fire_after_send(&self, req: &WebRequest, response: &WebResponse, duration_ms: u64);
    pub(crate) fn log(&self, level: LogLevel, message: &str, fields: &HashMap<String, String>);
}
```

Ordering rules for hooks:

- Hooks of the same type fire in registration order.
- `before_routing` hooks fire sequentially. The first one to return `Some`
  wins; subsequent hooks are skipped.
- `before_handler` hooks follow the same first-wins rule.
- `on_not_found`, `on_method_not_allowed`, and `on_handler_error` fire the last
  registered hook. This lets language bridges override the default with a
  single registration.
- `after_handler` hooks fire sequentially, each receiving the response
  produced by the previous hook.
- `after_send` hooks fire in registration order; errors are logged but do not
  affect the response.
- `on_log` hooks all receive every log event.

### WebApp

`WebApp` composes a `Router` and a `HookRegistry` into the full request
dispatch pipeline. It implements `Fn(HttpRequest) -> HttpResponse` so it can be
passed directly to `HttpServer::bind`.

```rust
pub struct WebApp {
    router: Router,
    hooks: HookRegistry,
}

impl WebApp {
    pub fn new() -> Self;

    // --- Route registration (delegates to Router) ---

    pub fn add(
        &mut self,
        method: impl Into<String>,
        pattern: &str,
        handler: impl Fn(&WebRequest) -> WebResponse + Send + Sync + 'static,
    );
    pub fn get(&mut self, pattern: &str, handler: impl ...);
    pub fn post(&mut self, pattern: &str, handler: impl ...);
    pub fn put(&mut self, pattern: &str, handler: impl ...);
    pub fn delete(&mut self, pattern: &str, handler: impl ...);
    pub fn patch(&mut self, pattern: &str, handler: impl ...);

    // --- Hook registration (delegates to HookRegistry) ---

    pub fn on_server_start(&mut self, hook: impl ...);
    pub fn on_server_stop(&mut self, hook: impl ...);
    pub fn on_connect(&mut self, hook: impl ...);
    pub fn on_disconnect(&mut self, hook: impl ...);
    pub fn before_routing(&mut self, hook: impl ...);
    pub fn on_not_found(&mut self, hook: impl ...);
    pub fn on_method_not_allowed(&mut self, hook: impl ...);
    pub fn before_handler(&mut self, hook: impl ...);
    pub fn on_handler_error(&mut self, hook: impl ...);
    pub fn after_handler(&mut self, hook: impl ...);
    pub fn after_send(&mut self, hook: impl ...);
    pub fn on_log(&mut self, hook: impl ...);

    // --- Application helpers ---

    /// Emit a structured log event through the hook registry.
    pub fn log(&self, level: LogLevel, message: &str, fields: &HashMap<String, String>);

    /// Process one HTTP request through the full pipeline.
    ///
    /// This is the entry point called by `WebServer` for every request.
    pub fn handle(&self, request: HttpRequest) -> HttpResponse;
}
```

### Request Dispatch Pipeline

The `handle` method implements this pipeline in order:

```text
1. Record start time (for after_send duration).

2. Parse query string from request target into a HashMap.

3. Build a partial WebRequest with empty route_params and the parsed
   query_params.

4. Fire HookRegistry::run_before_routing(partial_request).
   → If any hook returns Some(response), jump to step 12.

5. Call Router::lookup(method, path).
   → NotFound     → run_on_not_found    → jump to step 12.
   → MethodNotAllowed → run_on_method_not_allowed → jump to step 12.
   → Matched(m)   → continue.

6. Fill WebRequest::route_params from m.params.

7. Fire HookRegistry::run_before_handler(full_request).
   → If any hook returns Some(response), jump to step 11.

8. Call handler(full_request) inside std::panic::catch_unwind.
   → Ok(response)  → continue.
   → Err(panic)    → run_on_handler_error(full_request, message) → step 11.

9. WebResponse produced by handler → step 11.

10. (step 11) Fire HookRegistry::run_after_handler(full_request, response).
    → Produces final WebResponse.

12. Convert WebResponse to HttpResponse.

13. Fire HookRegistry::fire_after_send(full_request, response, elapsed_ms).
    (non-blocking, errors are discarded)

14. Return HttpResponse to transport layer.
```

The numbers above are chosen so step 12 is always "produce the HTTP response"
regardless of how we got there.

### WebServer

`WebServer` is a thin wrapper around `HttpServer` that accepts a `WebApp`.
Language packages typically do not need to use `WebServer` directly — they
expose their own server type that owns a `WebServer` internally.

```rust
pub struct WebServer<P> {
    inner: HttpServer<P>,
}

impl<P> WebServer<P>
where
    P: transport_platform::TransportPlatform,
{
    pub fn bind(
        platform: P,
        address: tcp_runtime::BindAddress,
        options: HttpServerOptions,
        app: Arc<WebApp>,
    ) -> Result<Self, tcp_runtime::PlatformError>;

    pub fn local_addr(&self) -> std::net::SocketAddr;
    pub fn stop_handle(&self) -> tcp_runtime::StopHandle;
    pub fn serve(&mut self) -> Result<(), tcp_runtime::PlatformError>;
}
```

Platform-specific convenience constructors follow the same pattern as
`HttpServer`:

```rust
// macOS / BSD:
impl WebServer<transport_platform::bsd::KqueueTransportPlatform> {
    pub fn bind_kqueue(addr: impl ToSocketAddrs, options: HttpServerOptions, app: Arc<WebApp>)
        -> Result<Self, PlatformError>;
}

// Linux:
impl WebServer<transport_platform::linux::EpollTransportPlatform> {
    pub fn bind_epoll(addr: impl ToSocketAddrs, options: HttpServerOptions, app: Arc<WebApp>)
        -> Result<Self, PlatformError>;
}

// Windows:
impl WebServer<transport_platform::windows::WindowsTransportPlatform> {
    pub fn bind_windows(addr: impl ToSocketAddrs, options: HttpServerOptions, app: Arc<WebApp>)
        -> Result<Self, PlatformError>;
}
```

## Hook Lifecycle Reference

This table summarises every hook, the moment it fires, and how its return value
is used.

| Hook                    | When                           | Return value          | Default if absent           |
|-------------------------|--------------------------------|-----------------------|-----------------------------|
| `on_server_start`       | Server bound, before accept()  | `()` — fire-and-forget | nothing                    |
| `on_server_stop`        | Event loop exited              | `()` — fire-and-forget | nothing                    |
| `on_connect`            | TCP connection accepted        | `()` — fire-and-forget | nothing                    |
| `on_disconnect`         | TCP connection closed          | `()` — fire-and-forget | nothing                    |
| `before_routing`        | Before route lookup            | `Option<WebResponse>` | continue pipeline           |
| `on_not_found`          | No matching route              | `WebResponse`         | 404 Not Found               |
| `on_method_not_allowed` | Route exists, wrong method     | `WebResponse`         | 405 Method Not Allowed      |
| `before_handler`        | After routing, before handler  | `Option<WebResponse>` | continue pipeline           |
| `on_handler_error`      | Handler panicked               | `WebResponse`         | 500 Internal Server Error   |
| `after_handler`         | Handler returned successfully  | `WebResponse`         | pass response through       |
| `after_send`            | Response handed to transport   | `()` — fire-and-forget | nothing                    |
| `on_log`                | Any internal or app log event  | `()` — all receive it | nothing (events dropped)   |

## Language Bridge Protocol

Language packages that embed `web-core` follow this pattern:

### Step 1: Build a WebApp

The bridge creates a `WebApp` and registers routes and hooks using the Rust API.
Route handlers and hook bodies are Rust closures that call back into the
language runtime.

For Ruby with a GVL:

```rust
let app = Arc::new(Mutex::new(WebApp::new()));

// Register a GET route. The closure body re-acquires the GVL
// before calling into Ruby, using the same pattern as conduit_native.
app.lock().get("/hello/:name", move |req| {
    let request_copy = req.clone();
    let response = call_into_ruby_with_gvl(owner, request_copy);
    response
});

// Register a logging hook. The closure must NOT block; it queues
// a log message for Ruby to drain on its own thread.
app.lock().on_log(move |level, message, _fields| {
    ruby_log_mailbox.push(format!("[{level:?}] {message}"));
});
```

For Python with a GIL the pattern is identical, using the Python C-API GIL
acquire functions.

### Step 2: Bind a WebServer

```rust
let server = WebServer::bind_kqueue("127.0.0.1:3000", options, Arc::new(app))?;
server.fire_server_start_hook();  // fires on_server_start hooks
server.serve()?;
server.fire_server_stop_hook();   // fires on_server_stop hooks
```

`fire_server_start_hook` and `fire_server_stop_hook` are separate from `serve`
because `serve` blocks until the server stops. The language bridge must call
the hooks at the right time.

### Step 3: Access route parameters from the language side

When a handler closure fires, it receives a `&WebRequest`. The bridge extracts
the fields it needs and converts them into the language's native types:

```rust
// In a Ruby bridge handler closure:
move |req: &WebRequest| -> WebResponse {
    let params = req.route_params.clone();       // HashMap<String, String>
    let query  = req.query_params.clone();        // HashMap<String, String>
    let body   = req.body().to_vec();
    let method = req.method().to_string();
    let path   = req.path().to_string();

    // Build the Rack/WSGI env hash in the language, call the app.
    let response = dispatch_to_language_app(method, path, params, query, body);
    response.into()
}
```

## Conduit Ruby Refactor

`conduit_native` currently:

1. Receives raw `HttpRequest` from `embeddable-http-server`.
2. Builds a Rack-like env hash in Rust.
3. Dispatches to the Ruby application via `rb_thread_call_with_gvl`.
4. Calls the Ruby `dispatch_request` method.
5. Parses the `[status, headers, body]` triple returned by Ruby.

After this refactor:

1. `conduit_native` creates a `WebApp` at load time.
2. Each `get`/`post`/… DSL call in Ruby's `Conduit.app {}` block registers a
   route on the `WebApp`.
3. The handler closure for each route builds a Rack env hash, acquires the GVL,
   calls into Ruby, and converts the response.
4. `conduit_native`'s `NativeServer` wraps a `WebServer` instead of an
   `HttpServer`.
5. Route matching logic in Rust (`match_route_native`) is retired; the `Router`
   handles it.

The Ruby API surface is unchanged. The Ruby tests continue to pass without
modification.

## Testing Strategy

### Unit tests (within the `web-core` crate)

- `Router::lookup` returns `Matched`, `NotFound`, `MethodNotAllowed` as
  expected.
- Named parameters are correctly extracted from matched routes.
- Routes are matched in registration order.
- Query strings are parsed correctly, including percent-encoded values and
  empty values.
- `WebResponse` builders produce correct status codes and headers.
- `HookRegistry` fires hooks in the documented order.
- A `before_routing` hook that returns `Some` short-circuits the handler.
- A panicking handler triggers `on_handler_error` instead of unwinding.
- `after_handler` hooks chain in order, each seeing the previous output.
- `on_not_found` and `on_method_not_allowed` produce correct defaults.

### Integration tests (within the `web-core` crate)

- Full round-trip via `WebServer::bind_*` on `127.0.0.1:0` (OS-assigned port).
- `GET /hello/Adhithya` matches `/hello/:name` and injects `name = Adhithya`.
- `POST /missing` returns 404 when no route matches.
- `DELETE /hello/Adhithya` returns 405 when only GET is registered.
- A `before_routing` hook can reject unauthenticated requests.
- An `after_handler` hook adds a `X-Request-Id` header to every response.
- An `on_log` hook captures internal log events.

## Crate Location

```text
code/packages/rust/web-core/
  Cargo.toml
  src/
    lib.rs
    router.rs
    request.rs
    response.rs
    hooks.rs
    app.rs
    server.rs
    query.rs   (query string parser)
  tests/
    web_core_test.rs
  README.md
  CHANGELOG.md
```

## Dependencies

```toml
[dependencies]
embeddable-http-server = { path = "../embeddable-http-server" }
http-core              = { path = "../http-core" }
tcp-runtime            = { path = "../tcp-runtime" }
transport-platform     = { path = "../transport-platform" }
```

No additional third-party dependencies beyond what `embeddable-http-server`
already pulls in.

## Phase Plan

### Phase 1: Core crate (this spec)

Implement `WebRequest`, `WebResponse`, `Router`, `HookRegistry`, `WebApp`,
`WebServer` with full unit and integration tests.

### Phase 2: conduit refactor

Refactor `conduit_native` to use `WebApp` and `WebServer`. The Ruby API stays
identical; only the native internals change. Retire `match_route_native`.

### Phase 3: Async promotion (WEB01)

Wire the `WebApp::handle` pipeline through
`embeddable-tcp-server::new_inprocess_mailbox()` so the HTTP I/O thread submits
work and returns immediately. Each language bridge provides a `worker_fn` that
receives `(JobRequest<HttpRequest>, AppHandle)` and calls
`AppHandle::handle(request)`.

This phase enables parallel request handling under the Rust thread pool without
requiring any change to the language-layer DSLs.

## Open Questions

- Should `WebApp::handle` be `&self` (shared, immutable after construction) or
  `Arc<WebApp>` cloned into each closure? Recommendation: `Arc<WebApp>` to
  keep the door open for async promotion.
- Should `Router` support wildcard segments (`:*rest`) for catch-all routes?
  Not needed for phase 1; reserve segment syntax for later.
- Should `after_send` hooks run on a detached thread to avoid adding latency to
  the request cycle? Yes, but thread spawning has overhead. Phase 1 runs them
  inline.
- Should `WebServer` also surface `on_connect`/`on_disconnect` hooks? These
  require `TcpRuntime` to support callbacks, which it currently does through
  the `on_close` callback. Phase 1 wires `on_disconnect` through the existing
  `on_close` hook; `on_connect` requires a new `TcpRuntime` callback.
