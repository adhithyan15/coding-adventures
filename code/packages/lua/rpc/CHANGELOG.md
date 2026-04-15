# Changelog — coding-adventures-rpc (Lua)

## [0.1.0] — 2026-04-11

### Added

- `coding_adventures.rpc` module: codec-agnostic RPC primitive for Lua.
- `RpcServer` — blocking read-dispatch-write loop over pluggable codec and framer.
  - `Server.new(codec, framer)` factory.
  - `Server:on_request(method, handler)` — chainable handler registration.
  - `Server:on_notification(method, handler)` — chainable handler registration.
  - `Server:serve()` — blocking loop; `pcall()` for handler error recovery.
  - Unknown request → `-32601 Method not found` response.
  - Unknown notification → silent drop, no response.
  - Handler `error()` → `-32603 Internal error` response with error message in `data`.
  - Codec decode failure → error response with `nil` id, continues loop.
- `RpcClient` — synchronous blocking request/response client.
  - `Client.new(codec, framer)` factory.
  - `Client:request(method, params)` — auto-incrementing id, blocking, returns `result, err`.
  - `Client:notify(method, params)` — fire-and-forget notification.
  - `Client:on_notification(method, handler)` — chainable server-push handler.
- Message constructors: `request_msg`, `response_msg`, `error_msg`, `notification_msg`.
- Error code constants: `PARSE_ERROR`, `INVALID_REQUEST`, `METHOD_NOT_FOUND`,
  `INVALID_PARAMS`, `INTERNAL_ERROR`.
- Comprehensive busted test suite (`tests/test_rpc.lua`) — 40+ test cases covering
  all specified behaviours.
- Literate inline comments explaining architecture, interface contracts, and
  design decisions.
