//! Lifecycle hook registry.
//!
//! `HookRegistry` holds zero or more listeners for each lifecycle event in the
//! request pipeline. All hooks are heap-allocated closures
//! (`Arc<dyn Fn + Send + Sync>`) so they can be registered from any thread and
//! remain alive for the lifetime of the server.
//!
//! Language bridges install hooks by wrapping their VM callbacks in these
//! closures. For GVL-constrained runtimes like Ruby, the closure body calls
//! `rb_thread_call_with_gvl` before invoking application code, exactly as
//! `conduit_native` does today.
//!
//! ## Ordering rules
//!
//! - `before_routing` and `before_handler` — first-wins short-circuit: the
//!   first hook to return `Some(response)` ends the chain; later hooks are
//!   skipped.
//! - `on_not_found`, `on_method_not_allowed`, `on_handler_error` — last-wins:
//!   only the most recently registered hook fires (so a bridge can override the
//!   default with one registration).
//! - `after_handler` — chained: each hook receives the response returned by the
//!   previous hook and may return a modified response.
//! - `after_send`, `on_connect`, `on_disconnect`, `on_server_start`,
//!   `on_server_stop`, `on_log` — all registered hooks fire in order.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::request::WebRequest;
use crate::response::WebResponse;

/// Severity level for log events emitted by `web-core` or application code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

// --- Hook type aliases ---

type ServerStartHook = Arc<dyn Fn(SocketAddr) + Send + Sync>;
type ServerStopHook = Arc<dyn Fn() + Send + Sync>;
type ConnectHook = Arc<dyn Fn(u64, SocketAddr) + Send + Sync>;
type DisconnectHook = Arc<dyn Fn(u64) + Send + Sync>;
type BeforeRoutingHook = Arc<dyn Fn(&WebRequest) -> Option<WebResponse> + Send + Sync>;
type NotFoundHook = Arc<dyn Fn(&WebRequest) -> WebResponse + Send + Sync>;
type MethodNotAllowedHook = Arc<dyn Fn(&WebRequest) -> WebResponse + Send + Sync>;
type BeforeHandlerHook = Arc<dyn Fn(&WebRequest) -> Option<WebResponse> + Send + Sync>;
type HandlerErrorHook = Arc<dyn Fn(&WebRequest, &str) -> WebResponse + Send + Sync>;
type AfterHandlerHook = Arc<dyn Fn(&WebRequest, WebResponse) -> WebResponse + Send + Sync>;
type AfterSendHook = Arc<dyn Fn(&WebRequest, &WebResponse, u64) + Send + Sync>;
type LogHook = Arc<dyn Fn(LogLevel, &str, &HashMap<String, String>) + Send + Sync>;

/// Lifecycle hook registry.
#[derive(Default)]
pub struct HookRegistry {
    on_server_start: Vec<ServerStartHook>,
    on_server_stop: Vec<ServerStopHook>,
    on_connect: Vec<ConnectHook>,
    on_disconnect: Vec<DisconnectHook>,
    before_routing: Vec<BeforeRoutingHook>,
    on_not_found: Vec<NotFoundHook>,
    on_method_not_allowed: Vec<MethodNotAllowedHook>,
    before_handler: Vec<BeforeHandlerHook>,
    on_handler_error: Vec<HandlerErrorHook>,
    after_handler: Vec<AfterHandlerHook>,
    after_send: Vec<AfterSendHook>,
    on_log: Vec<LogHook>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    // --- Registration ---

    /// Register a hook that fires once after the server socket is bound.
    pub fn on_server_start(&mut self, hook: impl Fn(SocketAddr) + Send + Sync + 'static) {
        self.on_server_start.push(Arc::new(hook));
    }

    /// Register a hook that fires once when the server event loop exits.
    pub fn on_server_stop(&mut self, hook: impl Fn() + Send + Sync + 'static) {
        self.on_server_stop.push(Arc::new(hook));
    }

    /// Register a hook that fires when a TCP connection is accepted.
    pub fn on_connect(&mut self, hook: impl Fn(u64, SocketAddr) + Send + Sync + 'static) {
        self.on_connect.push(Arc::new(hook));
    }

    /// Register a hook that fires when a TCP connection closes.
    pub fn on_disconnect(&mut self, hook: impl Fn(u64) + Send + Sync + 'static) {
        self.on_disconnect.push(Arc::new(hook));
    }

    /// Register a before-routing hook.
    ///
    /// Return `Some(response)` to short-circuit the rest of the pipeline.
    pub fn before_routing(
        &mut self,
        hook: impl Fn(&WebRequest) -> Option<WebResponse> + Send + Sync + 'static,
    ) {
        self.before_routing.push(Arc::new(hook));
    }

    /// Register a custom not-found handler (replaces the 404 default).
    pub fn on_not_found(&mut self, hook: impl Fn(&WebRequest) -> WebResponse + Send + Sync + 'static) {
        self.on_not_found.push(Arc::new(hook));
    }

    /// Register a custom method-not-allowed handler (replaces the 405 default).
    pub fn on_method_not_allowed(
        &mut self,
        hook: impl Fn(&WebRequest) -> WebResponse + Send + Sync + 'static,
    ) {
        self.on_method_not_allowed.push(Arc::new(hook));
    }

    /// Register a before-handler hook.
    ///
    /// Return `Some(response)` to short-circuit the handler.
    pub fn before_handler(
        &mut self,
        hook: impl Fn(&WebRequest) -> Option<WebResponse> + Send + Sync + 'static,
    ) {
        self.before_handler.push(Arc::new(hook));
    }

    /// Register a handler-error hook (fires when the handler panics).
    pub fn on_handler_error(
        &mut self,
        hook: impl Fn(&WebRequest, &str) -> WebResponse + Send + Sync + 'static,
    ) {
        self.on_handler_error.push(Arc::new(hook));
    }

    /// Register an after-handler hook for response transformation.
    pub fn after_handler(
        &mut self,
        hook: impl Fn(&WebRequest, WebResponse) -> WebResponse + Send + Sync + 'static,
    ) {
        self.after_handler.push(Arc::new(hook));
    }

    /// Register an after-send hook for logging and metrics.
    pub fn after_send(
        &mut self,
        hook: impl Fn(&WebRequest, &WebResponse, u64) + Send + Sync + 'static,
    ) {
        self.after_send.push(Arc::new(hook));
    }

    /// Register a structured log sink.
    pub fn on_log(
        &mut self,
        hook: impl Fn(LogLevel, &str, &HashMap<String, String>) + Send + Sync + 'static,
    ) {
        self.on_log.push(Arc::new(hook));
    }

    // --- Execution (called by WebApp) ---

    pub(crate) fn fire_server_start(&self, addr: SocketAddr) {
        for hook in &self.on_server_start {
            hook(addr);
        }
    }

    pub(crate) fn fire_server_stop(&self) {
        for hook in &self.on_server_stop {
            hook();
        }
    }

    // fire_connect and fire_disconnect are wired in Phase 3 when the async
    // promotion lands and tcp-runtime gains a connection-open callback.
    #[allow(dead_code)]
    pub(crate) fn fire_connect(&self, connection_id: u64, peer_addr: SocketAddr) {
        for hook in &self.on_connect {
            hook(connection_id, peer_addr);
        }
    }

    #[allow(dead_code)]
    pub(crate) fn fire_disconnect(&self, connection_id: u64) {
        for hook in &self.on_disconnect {
            hook(connection_id);
        }
    }

    /// Run before-routing hooks. Returns the first non-None response, or None.
    pub(crate) fn run_before_routing(&self, req: &WebRequest) -> Option<WebResponse> {
        for hook in &self.before_routing {
            if let Some(response) = hook(req) {
                return Some(response);
            }
        }
        None
    }

    /// Run the not-found handler. Falls back to 404 if none is registered.
    pub(crate) fn run_on_not_found(&self, req: &WebRequest) -> WebResponse {
        match self.on_not_found.last() {
            Some(hook) => hook(req),
            None => WebResponse::not_found(),
        }
    }

    /// Run the method-not-allowed handler. Falls back to 405 if none.
    pub(crate) fn run_on_method_not_allowed(&self, req: &WebRequest) -> WebResponse {
        match self.on_method_not_allowed.last() {
            Some(hook) => hook(req),
            None => WebResponse::method_not_allowed(),
        }
    }

    /// Run before-handler hooks. Returns the first non-None response, or None.
    pub(crate) fn run_before_handler(&self, req: &WebRequest) -> Option<WebResponse> {
        for hook in &self.before_handler {
            if let Some(response) = hook(req) {
                return Some(response);
            }
        }
        None
    }

    /// Run the handler-error handler. Falls back to 500 if none is registered.
    pub(crate) fn run_on_handler_error(&self, req: &WebRequest, error: &str) -> WebResponse {
        match self.on_handler_error.last() {
            Some(hook) => hook(req, error),
            None => WebResponse::internal_error(error),
        }
    }

    /// Run after-handler hooks, chaining the response through each one.
    pub(crate) fn run_after_handler(&self, req: &WebRequest, mut response: WebResponse) -> WebResponse {
        for hook in &self.after_handler {
            response = hook(req, response);
        }
        response
    }

    pub(crate) fn fire_after_send(&self, req: &WebRequest, response: &WebResponse, duration_ms: u64) {
        for hook in &self.after_send {
            hook(req, response, duration_ms);
        }
    }

    pub(crate) fn log(
        &self,
        level: LogLevel,
        message: &str,
        fields: &HashMap<String, String>,
    ) {
        for hook in &self.on_log {
            hook(level, message, fields);
        }
    }
}
