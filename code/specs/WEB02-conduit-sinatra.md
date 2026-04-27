# WEB02 — Conduit Sinatra DSL

## Overview

`conduit` is the Ruby package that exposes the `web-core` HTTP engine as a
Sinatra-style framework. Phase 1 (WEB00) built the engine; Phase 2 wired Ruby
routes into Rust. This spec covers Phase 3: completing the Sinatra surface area
so that `conduit` can be used as a production-grade Ruby web framework.

The goal is a DSL where a developer never touches Rack, never thinks about HTTP
framing, and gets the power of the Rust event loop plus idiomatic Ruby handlers.

```ruby
app = CodingAdventures::Conduit.app do
  set :app_name, "My App"

  before do |request|
    halt(503, "Maintenance") if request.path == "/down"
  end

  get "/" do
    html "<h1>Hello!</h1>"
  end

  get "/hello/:name" do |request|
    json({ message: "Hello #{params["name"]}" })
  end

  post "/echo" do |request|
    json(request.json)
  end

  get "/redirect" do
    redirect "/", 301
  end

  not_found do |request|
    html "<h1>Not Found: #{request.path}</h1>", 404
  end

  error do |request, _err|
    json({ error: "Internal Server Error" }, 500)
  end
end

server = CodingAdventures::Conduit::Server.new(app, host: "0.0.0.0", port: 3000)
server.serve
```

## Architecture: Rust vs. Ruby Split

The key design principle is that **Rust owns plumbing, Ruby owns logic**.

```text
┌──────────────────────────────────────────────────────────────┐
│  Sinatra DSL layer (Ruby)                                      │
│  Application  HandlerContext  HaltError  Request  Server       │
│  before/after filters · halt/redirect/json/html helpers        │
│  not_found/error handlers · settings store                     │
└──────────────────────────┬───────────────────────────────────┘
                           │  native_dispatch_route(i, env)
                           │  native_run_before_filters(env) → nil | [s,h,b]
                           │  native_run_after_filters(env, [s,h,b]) → [s,h,b]
                           │  native_run_not_found(env) → nil | [s,h,b]
                           │  native_run_error_handler(env, msg) → nil | [s,h,b]
┌──────────────────────────▼───────────────────────────────────┐
│  conduit_native (Rust)                                         │
│  Wraps web-core WebApp · registers hooks at init time          │
│  Acquires GVL before every Ruby call                           │
│  Detects HaltError signal: nil vs. [s,h,b] from Ruby          │
└──────────────────────────┬───────────────────────────────────┘
                           │  WebApp::handle(HttpRequest) → HttpResponse
┌──────────────────────────▼───────────────────────────────────┐
│  web-core  (WEB00)                                             │
│  Router · HookRegistry · WebApp · WebServer                    │
│  All 12 lifecycle hook points                                  │
└──────────────────────────────────────────────────────────────┘
```

**Rust owns:**
- Route matching and parameter extraction (via `web-core::Router`)
- HTTP/1 framing, TCP event loop (via `embeddable-http-server`, `tcp-runtime`)
- Hook dispatch pipeline (fires 12 lifecycle hooks in correct order)
- GVL management (`rb_thread_call_with_gvl`, `rb_thread_call_without_gvl`)
- Exception safety (`rb_protect` wraps every Ruby call)
- Signal protocol: Ruby returns `nil` (no short-circuit) or `[status, headers, body]`

**Ruby owns:**
- `HandlerContext` — instance-exec target for all blocks; defines `json`, `html`,
  `text`, `halt`, `redirect`
- `HaltError` — raised by context helpers to carry status+body+headers
- `Application` DSL — `before`, `after`, `not_found`, `error`, `set`
- `Request` — body parsing (`#json`, `#form`)
- All user-supplied logic (handler blocks, filter blocks)

## New Classes

### HaltError

`HaltError < StandardError` is the universal short-circuit mechanism.
Every response helper raises it; the dispatch layer catches it.

```ruby
class HaltError < StandardError
  attr_reader :status, :body, :halt_headers

  # status    — Integer HTTP status code (e.g. 200, 302, 404, 503)
  # body      — String response body (default "")
  # headers   — Hash or Array of [name, value] pairs (default {})
  def initialize(status, body = "", headers = {})
end
```

`halt_headers` is always a normalized `Array` of `[name, value]` String pairs,
regardless of whether a `Hash` or `Array` was supplied to the constructor.

`HaltError` never leaves the dispatch boundary. It is caught either in
`NativeServer#invoke_route` (for route handler blocks) or in
`NativeServer#native_run_before_filters` (for filter blocks). Rust never sees
it as an unhandled exception.

### HandlerContext

`HandlerContext` is the evaluation context for every block — route handlers,
before/after filters, not-found handlers, and error handlers.

```ruby
class HandlerContext
  attr_reader :request

  def initialize(request)

  # Shorthand for request.params (route-captured parameters).
  def params → Hash

  # Send a JSON response. Serializes data with JSON.generate and sets
  # content-type to application/json. Raises HaltError.
  def json(data, status = 200) → never returns

  # Send an HTML response. Raises HaltError.
  def html(content, status = 200) → never returns

  # Send a plain text response. Raises HaltError.
  def text(content, status = 200) → never returns

  # Short-circuit immediately with the given status, body, and headers.
  # Raises HaltError.
  def halt(status, body = "", headers = {}) → never returns

  # Redirect to url with the given status (default 302 Found).
  # Raises HaltError.
  def redirect(url, status = 302) → never returns
end
```

All blocks are called via `HandlerContext.new(request).instance_exec(request, &block)`.
This means:
- Blocks with `do |request| ... end` still work — the block argument is the request.
- Blocks with `do ... end` (arity 0) can access `request` as a method on `self`.
- `json(...)`, `html(...)`, `halt(...)`, `redirect(...)` are methods on `self`.
- There is no change to existing single-argument block signatures.

### Application — additions

```ruby
class Application
  # Existing methods unchanged: get, post, put, delete, patch, routes, call

  # Register a before filter. Runs for every request, before route lookup.
  # Matches Sinatra semantics: fires even when no route matches (useful for
  # maintenance mode, auth, rate limiting). Named route params are NOT
  # available here; use request.path and request.query_params.
  # Call halt() to short-circuit and send a response immediately.
  def before(&block)

  # Register an after filter. Runs after every route handler for side effects.
  # Cannot modify the response in this version.
  def after(&block)

  # Register a custom not-found handler.
  # Called when no route matches the request path.
  def not_found(&block)

  # Register a custom error handler.
  # Called when a route handler raises an exception or panics.
  # The block receives (request, error_message).
  def error(&block)

  # Store a configuration value.
  def set(key, value)

  # Read configuration values.
  attr_reader :settings

  # Exposed to Rust for hook registration checks.
  attr_reader :before_filters    # Array of Procs
  attr_reader :after_filters     # Array of Procs
  attr_reader :not_found_handler # Proc or nil
  attr_reader :error_handler     # Proc or nil
end
```

### Request — additions

```ruby
class Request
  # Existing methods unchanged: method, path, query_string, body, header,
  # content_length, content_type, [], env, params, query_params, headers

  # Parse the request body as JSON. Memoized. Raises JSON::ParserError if
  # the body is not valid JSON.
  def json → Hash | Array | String | Numeric | nil

  # Parse the request body as URL-encoded form data. Memoized.
  def form → Hash
end
```

### NativeServer — new dispatch methods

These are called by Rust via the GVL; they are not part of the public API.

```ruby
class NativeServer
  # Called by Rust for before filters. Returns nil if no filter halted;
  # returns [status, headers, body] if a filter called halt.
  def native_run_before_filters(env) → nil | [Integer, Array, Array]

  # Called by Rust for after filters. Returns the (possibly unchanged)
  # response as [status, headers, body].
  def native_run_after_filters(env, response) → [Integer, Array, Array]

  # Called by Rust when no route matches. Returns nil if no custom handler
  # is registered; returns [status, headers, body] otherwise.
  def native_run_not_found(env) → nil | [Integer, Array, Array]

  # Called by Rust when a handler raises. Returns nil if no custom handler
  # is registered; returns [status, headers, body] otherwise.
  def native_run_error_handler(env, error_message) → nil | [Integer, Array, Array]
end
```

## Rust Hook Registration Protocol

`conduit_native`'s `server_initialize` inspects the `Application` at startup
and registers hooks into `WebApp` only when needed. This avoids GVL overhead on
requests where no filters are configured.

```
server_initialize(self, app, host, port, max_connections):
  1. Iterate app.routes → register route closures (unchanged)

  2. if app.before_filters.length > 0:
       web_app.before_handler(|req| dispatch_before_filters_to_ruby(owner, req))
       # uses before_handler (fires after routing) so route params are available

  3. if app.after_filters.length > 0:
       web_app.after_handler(|req, resp| dispatch_after_filters_to_ruby(owner, req, resp))

  4. if app.not_found_handler != nil:
       web_app.on_not_found(|req| dispatch_not_found_to_ruby(owner, req))

  5. if app.error_handler != nil:
       web_app.on_handler_error(|req, error| dispatch_error_handler_to_ruby(owner, req, error))

  6. Bind server (unchanged)
```

### Hook dispatch protocol

Each dispatch function calls a corresponding Ruby method on the `NativeServer`
instance (`owner`). The Ruby method catches `HaltError` and returns a signal:

| Return value  | Meaning                      | Rust action                 |
|---------------|------------------------------|-----------------------------|
| `nil`         | No short-circuit / no handler | Continue or use default     |
| `[s, h, b]`  | Short-circuit with this response | Parse and return         |

This protocol means **Rust never needs to know about `HaltError`**. The Ruby
boundary converts exceptions into data before Rust sees them.

### `dispatch_before_filters_to_ruby`

Called from the `before_handler` hook. Returns `Option<WebResponse>`:

```
1. Clone WebRequest for GVL call.
2. rb_thread_call_with_gvl:
   a. build_env(request)
   b. rb_protect → NativeServer#native_run_before_filters(env)
   c. if exception: set error response, return
   d. if result == nil: call.result = None
   e. if result is [s,h,b]: call.result = Some(parse_web_response(result))
3. Return call.result (None = continue, Some(resp) = short-circuit).
```

### `dispatch_after_filters_to_ruby`

Called from the `after_handler` hook. Returns `WebResponse` (possibly modified):

```
1. Clone WebRequest + WebResponse.
2. rb_thread_call_with_gvl:
   a. build_env(request)
   b. web_response_to_rb_array(response)
   c. rb_protect → NativeServer#native_run_after_filters(env, response_rb)
   d. if exception: leave response unchanged, clear exception
   e. if result is [s,h,b]: replace call.response with parse_web_response(result)
3. Return call.response.
```

### `dispatch_not_found_to_ruby`

Called from the `on_not_found` hook. Returns `WebResponse`:

```
1. rb_thread_call_with_gvl:
   a. build_env(request)
   b. rb_protect → NativeServer#native_run_not_found(env)
   c. if exception or nil: return WebResponse::not_found()
   d. if result is [s,h,b]: return parse_web_response(result)
```

### `dispatch_error_handler_to_ruby`

Called from the `on_handler_error` hook. Returns `WebResponse`:

```
1. rb_thread_call_with_gvl:
   a. build_env(request)
   b. rb_protect → NativeServer#native_run_error_handler(env, error_message_rb)
   c. if exception or nil: return WebResponse::internal_error(error)
   d. if result is [s,h,b]: return parse_web_response(result)
```

## Halt / Short-circuit Flow

```text
GET /admin (before filter calls halt(403, "Forbidden")):

  WebApp::handle
    → HookRegistry::run_before_handler
      → dispatch_before_filters_to_ruby (acquires GVL)
        → NativeServer#native_run_before_filters(env)
          → HandlerContext#instance_exec(request, &filter)
            → filter raises HaltError(403, "Forbidden")
          ← HaltError caught in native_run_before_filters
          ← returns [403, [], ["Forbidden"]] to Rust
        ← Rust parses → Some(WebResponse{status:403, body:"Forbidden"})
      ← before_handler returns Some(response) → pipeline short-circuits
    ← request is NOT routed, handler NOT called
  ← HttpResponse{status:403, body:"Forbidden"} sent to client
```

```text
GET /hello (handler calls json({msg:"hello"})):

  WebApp::handle
    → Route matched → dispatch_route_to_ruby (acquires GVL)
      → NativeServer#native_dispatch_route(index, env)
        → HandlerContext.new(request).instance_exec(request, &block)
          → json({msg:"hello"}) raises HaltError(200, '{"msg":"hello"}', content-type: json)
        ← HaltError caught in native_dispatch_route
        ← returns [200, [["content-type","application/json"]], ['{"msg":"hello"}']]
      ← Rust parses → WebResponse{status:200, body:...}
    ← HttpResponse sent to client
```

## Response Helper Reference

| Helper          | Status | Content-Type         | Body            |
|-----------------|--------|----------------------|-----------------|
| `json(data)`    | 200    | application/json     | JSON.generate(data) |
| `json(data, n)` | n      | application/json     | JSON.generate(data) |
| `html(s)`       | 200    | text/html            | s               |
| `html(s, n)`    | n      | text/html            | s               |
| `text(s)`       | 200    | text/plain           | s               |
| `text(s, n)`    | n      | text/plain           | s               |
| `halt(n, b)`    | n      | (from headers param) | b               |
| `redirect(url)` | 302    | (none)               | (empty)         |
| `redirect(u,n)` | n      | (none)               | (empty)         |

String return values from handlers that do NOT call a helper are still
normalized by `normalize_result` with `content-type: text/plain; charset=utf-8`.

## Testing Strategy

### Pure Ruby unit tests (no server)

- `HaltError` stores status, body, and normalized header pairs.
- `HandlerContext#json` raises `HaltError` with correct status and content-type.
- `HandlerContext#html` raises `HaltError` with correct status and content-type.
- `HandlerContext#text` raises `HaltError` with correct status and content-type.
- `HandlerContext#halt` raises `HaltError` with exact status and body.
- `HandlerContext#redirect` raises `HaltError` with Location header.
- `HandlerContext#params` delegates to `request.params`.
- `Application#before` appends to `before_filters`.
- `Application#after` appends to `after_filters`.
- `Application#not_found` sets `not_found_handler`.
- `Application#error` sets `error_handler`.
- `Application#set` / `#settings` round-trip.
- `Request#json` parses JSON body.
- `Request#form` parses URL-encoded body.
- Before filter: fires and request is accessible.
- Before filter: `halt` short-circuits (handler block is NOT called).
- After filter: fires after handler.
- Settings round-trip in Application.

### End-to-end tests (real TCP server)

- `GET /` with `json({})` handler → 200 with `application/json` content-type.
- `GET /hello/:name` with `json({name: params["name"]})` → 200 JSON.
- Before filter halts on a specific path → handler is never called.
- Before filter passes for other paths → handler runs.
- Custom `not_found` handler → custom 404 body.
- Custom `error` handler → custom 500 body on handler raise.
- `POST /echo` with JSON body → handler reads `request.json` → JSON response.
- `GET /redirect` → 301 with Location header.
- After filter runs for all routes (side effect captured).

## Package Location

```text
code/packages/ruby/conduit/
  lib/coding_adventures/conduit/
    halt_error.rb         (new)
    handler_context.rb    (new)
    application.rb        (extended)
    request.rb            (extended)
    router.rb             (unchanged)
    route.rb              (unchanged)
    server.rb             (NativeServer extended)
    version.rb            (version bump)
  ext/conduit_native/src/lib.rs   (hook registration + dispatch functions)
  test/conduit_test.rb            (expanded)
```

## Dependencies

No new Ruby dependencies. The `json` standard library gem is already available
in Ruby's standard library. `uri` for `URI.decode_www_form` is also stdlib.

The Rust extension gains no new crate dependencies. All Rust changes are within
the `conduit_native` crate itself.

## Divergences from Classic Sinatra

This spec intentionally omits some Sinatra features that belong to later phases
or to application code:

- **Templating** (erb, haml): user code responsibility.
- **Sessions / cookies**: user code responsibility.
- **Middleware stack**: not implemented in this phase.
- **After-filter response modification**: after filters run for side effects only.
  Response modification may be added in a future phase.
- **Pattern-filtered before/after**: `before "/admin/*"` is not supported yet;
  all before/after filters run for every matched route.
- **Multiple error handlers by status code**: only one global error handler.
