# Conduit

`Conduit` is a Sinatra-style Ruby web framework backed by a Rust HTTP runtime.
Routing, connection management, and HTTP/1 framing live entirely in Rust via
`web-core`; Ruby only defines routes and executes handler blocks.

## How it works

```
Your Ruby app (handler blocks)
    ↓
Conduit DSL  (Application, Server, Request — pure Ruby)
    ↓
conduit_native  (Rust extension — routes registered in WebApp at init)
    ↓
web-core  (WebApp, WebServer, Router, lifecycle hooks)
    ↓
embeddable-http-server → tcp-runtime → kqueue / epoll / IOCP
```

When the server starts, Rust iterates `app.routes` and registers each route in
a `WebApp`. On every request the Rust router resolves the route and calls back to
`NativeServer#native_dispatch_route(route_index, env)` with a pre-built Rack env
that already contains `conduit.route_params` and `conduit.query_params`. Ruby
executes the handler block and returns `[status, headers, body]`.

## Quick start

```ruby
require "coding_adventures_conduit"

app = CodingAdventures::Conduit.app do
  get "/" do
    "Hello from Conduit!"
  end

  get "/hello/:name" do |request|
    "Hello #{request.params.fetch("name")}"
  end
end

server = CodingAdventures::Conduit::Server.new(app, port: 3000)
puts "Listening on http://localhost:#{server.port}"
server.serve   # blocks; Ctrl-C to stop
```

## Supported HTTP verbs

`get`, `post`, `put`, `delete`, `patch`

## Request object

| Method | Returns |
|--------|---------|
| `request.params` | Route params (`{ "name" => "Adhithya" }`) |
| `request.query_params` | Query string params |
| `request.headers` | Request headers (lowercase keys) |
| `request.body` | Request body string |
| `request.method` | HTTP verb |
| `request.path` | URL path |

## Response formats

A handler can return:

- A `String` — status 200, `text/plain; charset=utf-8`
- A `[status, headers_hash, body_array]` array — full Rack-style response
