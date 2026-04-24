# Changelog

## Unreleased

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
