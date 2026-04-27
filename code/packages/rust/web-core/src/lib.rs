//! # web-core
//!
//! A generic Rack/WSGI-like HTTP application layer built on
//! `embeddable-http-server`.
//!
//! `web-core` owns routing, request enrichment, response building, and a
//! lifecycle hook registry. Language packages — Ruby, Python, Lua, Perl, and
//! others — only need to implement their own application layer. All shared
//! plumbing lives here.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use web_core::{WebApp, WebResponse};
//!
//! let mut app = WebApp::new();
//!
//! app.get("/hello/:name", |req| {
//!     let name = req.route_params.get("name").map(|s| s.as_str()).unwrap_or("world");
//!     WebResponse::text(format!("Hello {name}"))
//! });
//!
//! app.after_handler(|_req, mut res| {
//!     res.headers.push(("X-Powered-By".into(), "web-core".into()));
//!     res
//! });
//!
//! // Wrap in Arc and hand to WebServer::bind_kqueue / bind_epoll / bind_windows.
//! let _app = Arc::new(app);
//! ```
//!
//! ## Layer map
//!
//! ```text
//! Language DSL (Ruby/conduit, Python, Lua, …)
//!     ↓
//! web-core  ← you are here
//!     ↓
//! embeddable-http-server
//!     ↓
//! tcp-runtime + transport-platform (kqueue / epoll / IOCP)
//! ```

pub mod app;
pub mod hooks;
pub mod query;
pub mod request;
pub mod response;
pub mod router;
pub mod server;

pub use app::WebApp;
pub use hooks::{HookRegistry, LogLevel};
pub use request::WebRequest;
pub use response::WebResponse;
pub use router::{Handler, Route, Router, RouteLookupResult, RouteMatch};
pub use server::WebServer;

pub const VERSION: &str = "0.1.0";
