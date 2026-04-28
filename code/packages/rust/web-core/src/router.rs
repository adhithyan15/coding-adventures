//! Route table and request matching.
//!
//! `Router` holds a list of `Route` values in registration order. The first
//! route whose pattern and method both match a request wins. Patterns use the
//! `RoutePattern` type from `http-core`: literal segments must match exactly,
//! `:param` segments match any single path segment and capture its value.
//!
//! Path matching strips the query string before comparing. Method comparison
//! is ASCII case-insensitive.
//!
//! When a path matches a registered pattern but the method is wrong,
//! `Router::lookup` returns `MethodNotAllowed` rather than `NotFound`. This
//! distinction lets the application return an accurate 405 instead of 404.

use std::sync::Arc;

use http_core::RoutePattern;

use crate::request::WebRequest;
use crate::response::WebResponse;

/// A handler function: takes a shared request reference and returns a response.
pub type Handler = Arc<dyn Fn(&WebRequest) -> WebResponse + Send + Sync + 'static>;

/// One registered route.
pub struct Route {
    /// HTTP method that this route responds to, stored uppercase.
    pub method: String,
    /// The parsed path pattern.
    pub pattern: RoutePattern,
    /// The application handler.
    pub handler: Handler,
}

/// Result of looking up a request in the router.
pub enum RouteLookupResult<'r> {
    /// A route matched both path and method.
    Matched(RouteMatch<'r>),
    /// The path matched a registered pattern but the method did not.
    MethodNotAllowed,
    /// No registered pattern matched the path at all.
    NotFound,
}

/// A successful route lookup: the matched route and extracted named params.
pub struct RouteMatch<'r> {
    /// The matched route (for access to the handler).
    pub route: &'r Route,
    /// Named path parameters in the order they appear in the pattern.
    pub params: Vec<(String, String)>,
}

/// Route table.
pub struct Router {
    routes: Vec<Route>,
}

impl Router {
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    /// Register a handler for the given method and path pattern.
    ///
    /// Method is stored in uppercase. Pattern is parsed into a `RoutePattern`.
    pub fn add(
        &mut self,
        method: impl Into<String>,
        pattern: &str,
        handler: impl Fn(&WebRequest) -> WebResponse + Send + Sync + 'static,
    ) {
        self.routes.push(Route {
            method: method.into().to_ascii_uppercase(),
            pattern: RoutePattern::parse(pattern),
            handler: Arc::new(handler),
        });
    }

    pub fn get(
        &mut self,
        pattern: &str,
        handler: impl Fn(&WebRequest) -> WebResponse + Send + Sync + 'static,
    ) {
        self.add("GET", pattern, handler);
    }

    pub fn post(
        &mut self,
        pattern: &str,
        handler: impl Fn(&WebRequest) -> WebResponse + Send + Sync + 'static,
    ) {
        self.add("POST", pattern, handler);
    }

    pub fn put(
        &mut self,
        pattern: &str,
        handler: impl Fn(&WebRequest) -> WebResponse + Send + Sync + 'static,
    ) {
        self.add("PUT", pattern, handler);
    }

    pub fn delete(
        &mut self,
        pattern: &str,
        handler: impl Fn(&WebRequest) -> WebResponse + Send + Sync + 'static,
    ) {
        self.add("DELETE", pattern, handler);
    }

    pub fn patch(
        &mut self,
        pattern: &str,
        handler: impl Fn(&WebRequest) -> WebResponse + Send + Sync + 'static,
    ) {
        self.add("PATCH", pattern, handler);
    }

    /// Find the first matching route for the given method and path.
    ///
    /// See `RouteLookupResult` for the three possible outcomes.
    pub fn lookup<'r>(&'r self, method: &str, path: &str) -> RouteLookupResult<'r> {
        let method_upper = method.to_ascii_uppercase();
        let mut path_matched = false;

        for route in &self.routes {
            if let Some(params) = route.pattern.match_path(path) {
                path_matched = true;
                if route.method == method_upper {
                    return RouteLookupResult::Matched(RouteMatch {
                        route,
                        params,
                    });
                }
            }
        }

        if path_matched {
            RouteLookupResult::MethodNotAllowed
        } else {
            RouteLookupResult::NotFound
        }
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}
