# WEB08 - Rust Conduit

## Overview

The earlier Conduit ports exposed the Rust `web-core` engine to Ruby, Python,
Lua, TypeScript, and Elixir. Rust itself still had only the lower engine crate.

`conduit` closes that gap with a Rust-native framework facade over `web-core`.
It is intentionally thin: route matching, request enrichment, lifecycle hooks,
HTTP/1 framing, and native event-loop dispatch remain owned by the existing Rust
runtime stack. The facade gives Rust applications the same small Conduit-shaped
surface that the language ports use.

## Where It Fits

```text
Rust application
    |
    v
conduit
  - Application
  - Server
  - response helpers
  - RequestExt helpers
    |
    v
web-core
  - WebApp
  - Router
  - HookRegistry
  - WebServer
    |
    v
embeddable-http-server
    |
    v
tcp-runtime + transport-platform
```

**Depends on:** WEB00 `web-core`.

**Used by:** Rust services, future Chief of Staff daemons, and any sandbox or
capability-caged app that wants a small in-repo HTTP surface.

## API

### Application

```rust
use conduit::RequestExt;

let mut app = conduit::Application::new();

app.set("app_name", "Conduit Hello");

app.before(|req| {
    if req.path() == "/down" {
        Some(conduit::halt(503, "Under maintenance"))
    } else {
        None
    }
});

app.get("/", |_| conduit::html("<h1>Hello from Conduit!</h1>"));

app.get("/hello/:name", |req| {
    conduit::text(format!("Hello {}", req.param("name").unwrap_or("world")))
});

app.not_found(|req| {
    conduit::html_status(404, format!("missing {}", req.path()))
});
```

Route helpers:

- `route(method, pattern, handler)`
- `get(pattern, handler)`
- `post(pattern, handler)`
- `put(pattern, handler)`
- `delete(pattern, handler)`
- `patch(pattern, handler)`

Hook helpers:

- `before`
- `after`
- `after_response`
- `not_found`
- `method_not_allowed`
- `on_error`
- `on_log`

### Server

```rust
let mut server = conduit::Server::bind("127.0.0.1", 3000, app)?;
server.serve()?;
```

`Server` selects the native `web-core` backend for the current platform:

- macOS/BSD: kqueue
- Linux: epoll
- Windows: IOCP

### Responses

Response helpers are dependency-free and return `web_core::WebResponse`:

- `text`
- `text_status`
- `html`
- `html_status`
- `json`
- `json_status`
- `redirect`
- `halt`

`json` accepts pre-serialized JSON bytes or text. The facade deliberately does
not pick a JSON serializer for Rust applications.

### Request Helpers

`RequestExt` adds:

- `param(name)`
- `query(name)`
- `body_text()`
- `body_text_lossy()`

## Demo Program

`code/programs/rust/conduit-hello` mirrors the existing Conduit hello demos:

- `GET /`
- `GET /hello/:name`
- `POST /echo`
- `GET /redirect`
- `GET /halt`
- `GET /down`
- `GET /error`
- custom 404 for `/missing`

## Test Strategy

The Rust facade must test:

1. response helper status and content-type behavior
2. named route parameter extraction through `RequestExt`
3. application settings and route introspection
4. before filter short-circuiting
5. observing after filters
6. custom not-found handling
7. panic recovery through `on_error`
8. query and body helpers
9. real TCP serving through the platform-native backend

The demo program must test that its route set stays aligned with the other
Conduit hello demos.
