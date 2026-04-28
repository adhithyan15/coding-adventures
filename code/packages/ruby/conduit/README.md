# Conduit

`Conduit` is a Sinatra-style Ruby web framework backed by a Rust HTTP runtime.
Routing, connection management, HTTP/1 framing, and lifecycle hooks live
entirely in Rust via `web-core`; Ruby defines routes and executes handler blocks.

## How it works

```
Your Ruby app (handler blocks, filters, settings)
    ↓
Conduit DSL  (Application, HandlerContext, Request, Server — pure Ruby)
    ↓
conduit_native  (Rust extension — routes and hooks registered in WebApp at init)
    ↓
web-core  (WebApp, WebServer, Router, HookRegistry, 12 lifecycle hook points)
    ↓
embeddable-http-server → tcp-runtime → kqueue / epoll / IOCP
```

At startup, `conduit_native` iterates `app.routes` and registers each route in
a `WebApp`. It also checks for `before`/`after` filters and `not_found`/`error`
handlers and registers the corresponding `web-core` hooks. On every request the
Rust pipeline dispatches route and hook callbacks to Ruby via the GVL.

## Quick start

```ruby
require "coding_adventures_conduit"

app = CodingAdventures::Conduit.app do
  set :app_name, "My App"

  before do |request|
    halt(503, "Under maintenance") if request.path == "/down"
  end

  get "/" do
    html "<h1>Hello from Conduit!</h1>"
  end

  get "/hello/:name" do |request|
    json({ message: "Hello #{request.params.fetch("name")}" })
  end

  post "/echo" do |request|
    json(request.json)
  end

  get "/old" do
    redirect "/", 301
  end

  not_found do |request|
    html "<h1>Not Found: #{request.path}</h1>", 404
  end

  error do |_request, err|
    json({ error: err }, 500)
  end
end

server = CodingAdventures::Conduit::Server.new(app, port: 3000)
puts "Listening on http://localhost:#{server.port}"
server.serve   # blocks; Ctrl-C to stop
```

## HTTP verbs

`get`, `post`, `put`, `delete`, `patch`

## Response helpers

All helpers raise `HaltError` to short-circuit the handler immediately:

| Helper | Status | Content-Type |
|--------|--------|--------------|
| `json(data, status=200)` | given | `application/json; charset=utf-8` |
| `html(content, status=200)` | given | `text/html; charset=utf-8` |
| `text(content, status=200)` | given | `text/plain; charset=utf-8` |
| `halt(status, body="", headers={})` | given | (from headers param) |
| `redirect(url, status=302)` | given | sets `Location` header |

A bare `String` return still works and is served as `200 text/plain; charset=utf-8`.

## Before / after filters

```ruby
before do |request|           # fires for EVERY request, before route lookup
  halt(401) unless authorized?(request)
end

after do |request|            # fires after every matched route (side effects)
  log_request(request)
end
```

Before filters use `web-core`'s `before_routing` hook and fire even when no
route matches (matching Sinatra semantics). Named route params are NOT available;
use `request.path` and `request.query_params`.

## Not-found and error handlers

```ruby
not_found do |request|
  html "<h1>404 Not Found</h1>", 404
end

error do |request, error_message|
  json({ error: error_message }, 500)
end
```

## Request object

| Method | Returns |
|--------|---------|
| `request.params` | Route params e.g. `{ "name" => "Adhithya" }` |
| `request.query_params` | Query string params |
| `request.headers` | Request headers (lowercase keys) |
| `request.body` | Request body as String |
| `request.json` | Body parsed as JSON (memoized) |
| `request.form` | Body parsed as URL-encoded form data (memoized) |
| `request.method` | HTTP verb |
| `request.path` | URL path |
| `request.content_type` | `Content-Type` header |
| `request.content_length` | `Content-Length` as Integer or nil |
| `request.header(name)` | First value of a named header (case-insensitive) |

## Settings

```ruby
app = Conduit.app do
  set :port, 3000
  set :environment, :production
end

app.settings[:port]        # => 3000
```

## Server

```ruby
server = CodingAdventures::Conduit::Server.new(app,
  host: "0.0.0.0",
  port: 3000,
  max_connections: 1024
)

server.start          # start in background thread, returns thread
server.serve          # block in current thread
server.stop           # signal graceful shutdown
server.close          # stop + wait + dispose
server.running?       # → true | false
server.local_addr     # → "127.0.0.1:3000"
server.port           # → 3000
```

## Architecture: Rust vs. Ruby split

**Rust owns:** route matching, HTTP/1 framing, TCP event loop, GVL management,
hook dispatch pipeline, exception safety (`rb_protect` wraps every Ruby call).

**Ruby owns:** handler blocks, filter blocks, response helpers, body parsing,
settings, all application logic.

The protocol is simple: Ruby returns either a `[status, headers, body]` triplet
(use this response) or `nil` (no short-circuit / use default). `HaltError` is
caught in Ruby before Rust ever sees it.
