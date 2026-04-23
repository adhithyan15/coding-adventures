# web-core

A generic Rack/WSGI-like HTTP application layer for the Rust native runtime stack.

`web-core` sits between `embeddable-http-server` (HTTP/1 framing and the native
event loop) and language bridges (Ruby, Python, Lua, Perl, …). It owns routing,
request enrichment, response building, and a lifecycle hook registry so that
every language that embeds the Rust runtime gets the same set of features
without reimplementing them.

## Layer map

```
Language DSL (Ruby/conduit, Python, Lua, …)
    ↓
web-core          ← you are here
    ↓
embeddable-http-server
    ↓
tcp-runtime + transport-platform (kqueue / epoll / IOCP)
```

## What it provides

| Concern              | Provided by |
|----------------------|-------------|
| Route table          | `Router` — `RoutePattern` from `http-core`, first-match wins |
| Request enrichment   | `WebRequest` — route params, query params, path split |
| Response building    | `WebResponse` — fluent builder, converts to `HttpResponse` |
| Lifecycle hooks      | `HookRegistry` — 12 hook points, `Arc<dyn Fn>` closures |
| Dispatch pipeline    | `WebApp::handle` — full lifecycle in one call |
| Server binding       | `WebServer` — thin wrapper around `HttpServer` |

## Quick start

```rust
use std::sync::Arc;
use web_core::{WebApp, WebResponse};
use embeddable_http_server::HttpServerOptions;

let mut app = WebApp::new();

app.get("/hello/:name", |req| {
    let name = req.route_params.get("name").map(|s| s.as_str()).unwrap_or("world");
    WebResponse::text(format!("Hello {name}"))
});

// Add a CORS header to every response.
app.after_handler(|_req, mut res| {
    res.headers.push(("Access-Control-Allow-Origin".into(), "*".into()));
    res
});

// Log every request.
app.after_send(|req, res, elapsed_ms| {
    eprintln!("{} {} → {} ({elapsed_ms}ms)", req.method(), req.path(), res.status);
});

let app = Arc::new(app);

// macOS / BSD:
let mut server = web_core::WebServer::bind_kqueue("127.0.0.1:3000", HttpServerOptions::default(), app)?;
println!("listening on {}", server.local_addr());
server.serve()?;
```

## Hook points

| Hook                    | When                              | Return / effect         |
|-------------------------|-----------------------------------|-------------------------|
| `on_server_start`       | After bind, before first accept   | fire-and-forget         |
| `on_server_stop`        | After event loop exits            | fire-and-forget         |
| `on_connect`            | TCP connection accepted           | fire-and-forget         |
| `on_disconnect`         | TCP connection closed             | fire-and-forget         |
| `before_routing`        | Before route lookup               | `Some(res)` short-circuits |
| `on_not_found`          | No route matched                  | replaces 404 default    |
| `on_method_not_allowed` | Path matched, wrong method        | replaces 405 default    |
| `before_handler`        | After routing, before handler     | `Some(res)` short-circuits |
| `on_handler_error`      | Handler panicked                  | replaces 500 default    |
| `after_handler`         | Handler returned successfully     | chains the response     |
| `after_send`            | Response handed to transport      | fire-and-forget         |
| `on_log`                | Any log event                     | all hooks receive it    |

## Dependencies

- `embeddable-http-server` — HTTP/1 framing, `HttpRequest` / `HttpResponse`
- `http-core` — `RoutePattern`, `RequestHead`, shared HTTP types
- `tcp-runtime` — `StopHandle`, `PlatformError`, `BindAddress`
- `transport-platform` — kqueue / epoll / IOCP abstractions

## Testing

```
cargo test -p web-core
```

37 tests: 8 unit tests (query string parser), 20 pipeline unit tests
(hook ordering, route matching, panic recovery), 9 end-to-end tests
(real TCP socket on port 0).

## Status

Phase 1 (this crate): complete.
Phase 2: conduit Ruby refactor — see `WEB00-web-core.md`.
Phase 3: async promotion via `embeddable-tcp-server::new_inprocess_mailbox`.
