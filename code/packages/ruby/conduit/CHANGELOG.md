# Changelog

## Unreleased

### Phase 3 — Sinatra DSL (WEB02)

- **`HaltError`** — new exception class carrying `status`, `body`, and
  `halt_headers`. Raised by all response helpers; caught by the dispatch layer
  before Rust ever sees it.

- **`HandlerContext`** — new evaluation context for handler and filter blocks.
  All blocks are now `instance_exec`'d inside a fresh `HandlerContext`, giving
  every block access to `json`, `html`, `text`, `halt`, `redirect`, and `params`
  without an explicit receiver. Existing blocks with `do |request|` still work
  unchanged — `instance_exec` passes the request as the block argument.

- **`Application#before`** — register before filters that run for every request
  before route lookup (uses `web-core`'s `before_routing` hook). Fires even
  when no route matches, matching Sinatra semantics.

- **`Application#after`** — register after filters for side effects (logging,
  metrics) that run after every matched route handler.

- **`Application#not_found`** — override the default 404 response.

- **`Application#error`** — override the default 500 response when a handler raises.

- **`Application#set` / `#settings`** — simple configuration store.

- **`Request#json`** — parse the request body as JSON (memoized).

- **`Request#form`** — parse the request body as URL-encoded form data (memoized).

- **`conduit_native` hook registration** — `server_initialize` now inspects the
  `Application` for filters and handlers; `before_routing`, `after_handler`,
  and `on_not_found` hooks are registered in `WebApp` only when needed.

- **In-GVL error dispatch** — when a route handler raises a Ruby exception,
  `dispatch_route_with_gvl` calls the custom error handler directly in the same
  GVL slot (avoids a redundant `rb_thread_call_with_gvl` round-trip).

- **`conduit-hello` upgraded** to a full Sinatra demo with 8 routes exercising
  all new features: before filter, after filter, JSON response, POST echo,
  redirect, halt, custom not-found, custom error handler.

- **44 tests** (up from 7); coverage now includes `HaltError`, `HandlerContext`,
  all filter types, body parsing, settings, and 12 end-to-end server tests.

### Phase 2 — Rust-side routing via `web-core`

- **Routing moved to Rust.** `NativeServer` now registers all routes from `app.routes`
  into a `web_core::WebApp` at initialisation time. The Rust router handles every
  `RouteLookupResult` (matched, not-found, method-not-allowed) without ever returning
  to Ruby for routing logic.
- **New `native_dispatch_route(route_index, env)` callback.** When a route matches,
  Rust calls this Ruby method with a pre-built Rack env hash that already includes
  `conduit.route_params` (populated by the Rust router) and `conduit.query_params`.
  Ruby only executes the handler block.
- **`NativeServer.new` arity changed** from `(host, port, max_connections)` to
  `(app, host, port, max_connections)`. `attach_app` and `dispatch_request` removed.
- **`Application` gains `routes` accessor** and HTTP verb helpers `post`, `put`,
  `delete`, `patch`.
- **`Route#match?`** simplified — `match_route_native` fallback removed; routing no
  longer duplicated between Ruby and Rust.
- **`normalize_result` in `NativeServer`** now returns `"text/plain; charset=utf-8"`
  for bare String responses (more correct than the previous `"text/plain"`).
- Added `conduit-hello` demo program (`code/programs/ruby/conduit-hello/`) showing
  Sinatra-style `GET /hello/:name` in ~30 lines of Ruby.

### Phase 1 (initial release)

- Add the first `Conduit` Rack-like Ruby package on top of the Rust HTTP runtime.
