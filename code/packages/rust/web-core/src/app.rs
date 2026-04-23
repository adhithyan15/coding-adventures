//! `WebApp`: the full request dispatch pipeline.
//!
//! `WebApp` composes a `Router` and a `HookRegistry` and implements the
//! pipeline described in WEB00-web-core.md. `WebServer` (and language bridges
//! that embed their own server) pass every `HttpRequest` to `WebApp::handle`,
//! which runs the full lifecycle and returns an `HttpResponse`.
//!
//! ## Pipeline
//!
//! ```text
//! HttpRequest arrives
//!   → parse query string, split path
//!   → build partial WebRequest (no route_params yet)
//!   → run before_routing hooks  →  short-circuit if Some(response)
//!   → Router::lookup(method, path)
//!       NotFound          → run on_not_found hook
//!       MethodNotAllowed  → run on_method_not_allowed hook
//!       Matched           → fill route_params
//!           → run before_handler hooks  →  short-circuit if Some(response)
//!           → call handler inside catch_unwind
//!               panic  → run on_handler_error hook
//!               ok     → response
//!   → run after_handler hooks (chain)
//!   → convert WebResponse → HttpResponse
//!   → fire after_send hooks (fire-and-forget)
//!   → return HttpResponse
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use embeddable_http_server::HttpRequest;
use embeddable_http_server::HttpResponse;

use crate::hooks::{HookRegistry, LogLevel};
use crate::query::{parse_query_string, split_target};
use crate::request::WebRequest;
use crate::response::WebResponse;
use crate::router::{Router, RouteLookupResult};

/// Generic Rack/WSGI-like HTTP application.
///
/// Build one with `WebApp::new()`, register routes and hooks, then wrap it in
/// an `Arc` and pass it to `WebServer::bind_*`.
pub struct WebApp {
    router: Router,
    hooks: HookRegistry,
}

impl WebApp {
    pub fn new() -> Self {
        Self {
            router: Router::new(),
            hooks: HookRegistry::new(),
        }
    }

    // --- Route registration ---

    pub fn add(
        &mut self,
        method: impl Into<String>,
        pattern: &str,
        handler: impl Fn(&WebRequest) -> WebResponse + Send + Sync + 'static,
    ) {
        self.router.add(method, pattern, handler);
    }

    pub fn get(
        &mut self,
        pattern: &str,
        handler: impl Fn(&WebRequest) -> WebResponse + Send + Sync + 'static,
    ) {
        self.router.get(pattern, handler);
    }

    pub fn post(
        &mut self,
        pattern: &str,
        handler: impl Fn(&WebRequest) -> WebResponse + Send + Sync + 'static,
    ) {
        self.router.post(pattern, handler);
    }

    pub fn put(
        &mut self,
        pattern: &str,
        handler: impl Fn(&WebRequest) -> WebResponse + Send + Sync + 'static,
    ) {
        self.router.put(pattern, handler);
    }

    pub fn delete(
        &mut self,
        pattern: &str,
        handler: impl Fn(&WebRequest) -> WebResponse + Send + Sync + 'static,
    ) {
        self.router.delete(pattern, handler);
    }

    pub fn patch(
        &mut self,
        pattern: &str,
        handler: impl Fn(&WebRequest) -> WebResponse + Send + Sync + 'static,
    ) {
        self.router.patch(pattern, handler);
    }

    // --- Hook registration ---

    pub fn on_server_start(&mut self, hook: impl Fn(std::net::SocketAddr) + Send + Sync + 'static) {
        self.hooks.on_server_start(hook);
    }

    pub fn on_server_stop(&mut self, hook: impl Fn() + Send + Sync + 'static) {
        self.hooks.on_server_stop(hook);
    }

    pub fn on_connect(
        &mut self,
        hook: impl Fn(u64, std::net::SocketAddr) + Send + Sync + 'static,
    ) {
        self.hooks.on_connect(hook);
    }

    pub fn on_disconnect(&mut self, hook: impl Fn(u64) + Send + Sync + 'static) {
        self.hooks.on_disconnect(hook);
    }

    pub fn before_routing(
        &mut self,
        hook: impl Fn(&WebRequest) -> Option<WebResponse> + Send + Sync + 'static,
    ) {
        self.hooks.before_routing(hook);
    }

    pub fn on_not_found(
        &mut self,
        hook: impl Fn(&WebRequest) -> WebResponse + Send + Sync + 'static,
    ) {
        self.hooks.on_not_found(hook);
    }

    pub fn on_method_not_allowed(
        &mut self,
        hook: impl Fn(&WebRequest) -> WebResponse + Send + Sync + 'static,
    ) {
        self.hooks.on_method_not_allowed(hook);
    }

    pub fn before_handler(
        &mut self,
        hook: impl Fn(&WebRequest) -> Option<WebResponse> + Send + Sync + 'static,
    ) {
        self.hooks.before_handler(hook);
    }

    pub fn on_handler_error(
        &mut self,
        hook: impl Fn(&WebRequest, &str) -> WebResponse + Send + Sync + 'static,
    ) {
        self.hooks.on_handler_error(hook);
    }

    pub fn after_handler(
        &mut self,
        hook: impl Fn(&WebRequest, WebResponse) -> WebResponse + Send + Sync + 'static,
    ) {
        self.hooks.after_handler(hook);
    }

    pub fn after_send(
        &mut self,
        hook: impl Fn(&WebRequest, &WebResponse, u64) + Send + Sync + 'static,
    ) {
        self.hooks.after_send(hook);
    }

    pub fn on_log(
        &mut self,
        hook: impl Fn(LogLevel, &str, &HashMap<String, String>) + Send + Sync + 'static,
    ) {
        self.hooks.on_log(hook);
    }

    // --- Application helpers ---

    /// Emit a structured log event through the hook registry.
    pub fn log(&self, level: LogLevel, message: &str, fields: &HashMap<String, String>) {
        self.hooks.log(level, message, fields);
    }

    /// Fire the on_server_start hooks. Call this after the socket is bound but
    /// before calling `serve`.
    pub fn fire_server_start(&self, addr: std::net::SocketAddr) {
        self.hooks.fire_server_start(addr);
    }

    /// Fire the on_server_stop hooks. Call this after `serve` returns.
    pub fn fire_server_stop(&self) {
        self.hooks.fire_server_stop();
    }

    // --- Request dispatch ---

    /// Process one HTTP request through the full lifecycle pipeline.
    ///
    /// This method is the sole entry point called by `WebServer` (or a
    /// language bridge) for every incoming request.
    pub fn handle(&self, request: HttpRequest) -> HttpResponse {
        let start = Instant::now();

        // Parse the request target into path and query string.
        let (path, query_str) = split_target(request.target());
        let path = path.to_string();
        let query_params = parse_query_string(query_str);
        let method = request.method().to_string();

        // Build a partial WebRequest (no route_params yet).
        let partial = WebRequest::new(
            request,
            path.clone(),
            HashMap::new(),
            query_params.clone(),
        );

        // before_routing hooks — first winner short-circuits.
        if let Some(early) = self.hooks.run_before_routing(&partial) {
            let response = self.hooks.run_after_handler(&partial, early);
            let elapsed = start.elapsed().as_millis() as u64;
            self.hooks.fire_after_send(&partial, &response, elapsed);
            return response.into();
        }

        // Route lookup.
        let matched = match self.router.lookup(&method, &path) {
            RouteLookupResult::NotFound => {
                let not_found = self.hooks.run_on_not_found(&partial);
                let response = self.hooks.run_after_handler(&partial, not_found);
                let elapsed = start.elapsed().as_millis() as u64;
                self.hooks.fire_after_send(&partial, &response, elapsed);
                return response.into();
            }
            RouteLookupResult::MethodNotAllowed => {
                let mna = self.hooks.run_on_method_not_allowed(&partial);
                let response = self.hooks.run_after_handler(&partial, mna);
                let elapsed = start.elapsed().as_millis() as u64;
                self.hooks.fire_after_send(&partial, &response, elapsed);
                return response.into();
            }
            RouteLookupResult::Matched(m) => m,
        };

        // Build full WebRequest with route params.
        let route_params: HashMap<String, String> = matched.params.into_iter().collect();
        let full_request = WebRequest::new(
            partial.http,
            path,
            route_params,
            query_params,
        );

        // before_handler hooks — first winner short-circuits.
        if let Some(early) = self.hooks.run_before_handler(&full_request) {
            let response = self.hooks.run_after_handler(&full_request, early);
            let elapsed = start.elapsed().as_millis() as u64;
            self.hooks.fire_after_send(&full_request, &response, elapsed);
            return response.into();
        }

        // Call the route handler, catching panics.
        let handler = Arc::clone(&matched.route.handler);
        let handler_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            handler(&full_request)
        }));

        let web_response = match handler_result {
            Ok(response) => response,
            Err(panic) => {
                let message = panic
                    .downcast_ref::<&str>()
                    .copied()
                    .or_else(|| panic.downcast_ref::<String>().map(|s| s.as_str()))
                    .unwrap_or("handler panicked");
                self.hooks.run_on_handler_error(&full_request, message)
            }
        };

        // after_handler hooks chain.
        let final_response = self.hooks.run_after_handler(&full_request, web_response);
        let elapsed = start.elapsed().as_millis() as u64;
        self.hooks.fire_after_send(&full_request, &final_response, elapsed);

        final_response.into()
    }
}

impl Default for WebApp {
    fn default() -> Self {
        Self::new()
    }
}
