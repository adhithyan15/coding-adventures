# conduit

Rust-native Conduit web framework facade over `web-core`.

This is the Rust sibling of the Ruby, Python, Lua, TypeScript, and Elixir
Conduit ports. The lower HTTP engine still lives in `web-core`; this crate gives
Rust users the small framework-shaped API directly.

## Quick Start

```rust
use conduit::{html, Application, RequestExt, Server};

let mut app = Application::new();

app.before(|req| {
    if req.path() == "/down" {
        Some(conduit::halt(503, "Under maintenance"))
    } else {
        None
    }
});

app.get("/", |_| html("<h1>Hello from Conduit!</h1>"));

app.get("/hello/:name", |req| {
    let name = req.param("name").unwrap_or("world");
    conduit::text(format!("Hello {name}"))
});

let mut server = Server::bind("127.0.0.1", 3000, app)?;
server.serve()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## API Surface

- `Application` — route registration, settings, before/after hooks, custom 404,
  custom 405, and panic recovery hooks
- `Server` — platform-native server binding over `web-core::WebServer`
- `RequestExt` — convenience accessors for route params, query params, and body text
- response helpers — `text`, `html`, `json`, `redirect`, `halt`, and explicit-status
  variants
- `escape_json_string` — tiny helper for dependency-free hand-built JSON

## Design

`conduit` deliberately does not fork routing, HTTP parsing, or lifecycle behavior.
It delegates those to:

- `web-core` for routing, request enrichment, hooks, and dispatch
- `embeddable-http-server` for HTTP/1 request/response framing
- `tcp-runtime` and `transport-platform` for native event-loop backends

The facade exists so Rust applications can use the same conceptual Conduit surface
without going through a language bridge or native extension.

## Development

```bash
bash BUILD
```
