# Changelog — web-core

## 0.1.0 (2026-04-23)

Initial release.

### Added

- `WebRequest` — enriched HTTP request with pre-parsed `route_params`,
  `query_params`, and `path` (query string stripped). Wraps `HttpRequest`
  from `embeddable-http-server`.
- `WebResponse` — fluent response builder with helpers `ok`, `text`, `json`,
  `not_found`, `method_not_allowed`, `internal_error`, `new`,
  `with_header`, `with_content_type`. Converts to/from `HttpResponse`.
- `Router` — route table with `add`, `get`, `post`, `put`, `delete`, `patch`.
  Uses `RoutePattern` from `http-core`. `lookup` returns `Matched`,
  `NotFound`, or `MethodNotAllowed`. First-registered route wins on overlap.
  Method comparison is ASCII case-insensitive.
- `HookRegistry` — 12 lifecycle hook points (`on_server_start`,
  `on_server_stop`, `on_connect`, `on_disconnect`, `before_routing`,
  `on_not_found`, `on_method_not_allowed`, `before_handler`,
  `on_handler_error`, `after_handler`, `after_send`, `on_log`).
  All hooks are `Arc<dyn Fn + Send + Sync + 'static>`.
- `WebApp` — composes `Router` and `HookRegistry`. `handle(HttpRequest)`
  runs the full dispatch pipeline including panic recovery via
  `std::panic::catch_unwind`.
- `WebServer` — thin wrapper around `HttpServer` with platform-specific
  constructors (`bind_kqueue`, `bind_epoll`, `bind_windows`). Fires
  `on_server_start` hooks after bind and `on_server_stop` hooks after
  `serve` returns.
- Query string parser (`query` module): percent-decoding (`%HH`), plus-as-space,
  empty keys skipped, duplicate keys keep last value.
- 37 tests: 8 query-parser unit tests, 20 hook/pipeline tests via
  `WebApp::handle`, 9 end-to-end tests over real TCP connections.
