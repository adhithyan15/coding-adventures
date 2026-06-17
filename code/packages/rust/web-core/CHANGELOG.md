# Changelog — web-core

## 0.3.1 (2026-06-16)

### Added

- **WEB01b-3 — comparative serving-mode benchmark** (`#[ignore]`d, in
  `tests/web_core_test.rs`): `web_serving_modes_cpu_bound_comparison` runs the
  same CPU-bound load across all three WEB01 serving modes — single-reactor
  `WebServer`, `ShardedWebServer` (parallel by connection), and
  `MailboxWebServer` (parallel by request) — and prints a wall-clock + speedup
  table so the tradeoffs are documented with data ("when to pick which mode").
  Like the sharded benchmark it is a runnable *measurement*, not a CI gate
  (the deterministic concurrency tests remain the CI proofs); it asserts only
  that both parallel modes beat the single reactor when ≥ 2 cores are present.
  Sample (14 cores): single 180ms, sharded 5.5×, mailbox 6.6×. See
  `code/specs/WEB01b-mailbox-parallelism.md`.

## 0.3.0 (2026-06-16)

### Added

- **`MailboxWebServer` (WEB01b-2)** — a per-request-parallel counterpart to
  `WebServer`, wrapping `embeddable-http-server`'s `MailboxHttpServer`. Where
  `ShardedWebServer` parallelises *by connection* (N reactors, inline dispatch),
  `MailboxWebServer` parallelises *by request*: a single reactor frames each
  request and submits it to a `worker_count`-thread pool; a worker runs
  `WebApp::handle` off the reactor thread and the pool's response router writes
  the reply back. The `Arc<WebApp>` is shared across pool threads unchanged
  (`handle` is `&self` + `Send + Sync`). `on_server_start` / `on_server_stop`
  fire once, like `WebServer`.
- Single cross-platform `bind(host, port, options, worker_count, app)` (the
  underlying `EmbeddableTcpServer` selects kqueue/epoll/IOCP internally — no
  per-OS constructors), returning `std::io::Result` (the mailbox stack is
  `io::Error`-based). Plus `local_addr` / `is_running` / `stop` / `serve`; the
  type is `Clone` so a clone can serve on a worker thread while another stops it.
- **Opt-in**, like `ShardedWebServer`: the default `WebServer` is untouched.
  Correct and in order for one-request-and-close and *sequential* keep-alive;
  pipelined-connection gating/reordering is WEB01b-1b. Exercised via conduit's
  `mailbox_server_dispatches_handlers_concurrently` (deterministic ≥ 2 in-flight
  proof). See `code/specs/WEB01b-mailbox-parallelism.md`.

## 0.2.0 (2026-06-16)

### Added

- **`ShardedWebServer` (WEB01a-2)** — a parallel counterpart to `WebServer`. It
  runs `WebApp::handle` across `worker_count` reactor shards (via
  `embeddable-http-server`'s `ShardedHttpServer`), so requests on different
  connections are dispatched concurrently. The same `Arc<WebApp>` is shared
  across all shards (immutable after construction — no locking; `handle` is
  `&self` + `Send + Sync`). Platform constructors `bind_kqueue_sharded` /
  `bind_epoll_sharded` / `bind_windows_sharded`, plus `serve` / `local_addr` /
  `worker_count` / `stop_handle`; `on_server_start` / `on_server_stop` fire once,
  exactly like `WebServer`.
- This is **opt-in**: the single-reactor `WebServer` is unchanged and remains the
  default. Hook/routing/`halt` semantics are identical (they run inside
  `app.handle` on the owning shard); HTTP/1.1 per-connection response ordering is
  preserved because each connection stays on one shard.
- Tests: `sharded_web_server_handles_requests_concurrently` deterministically
  proves cross-shard parallelism (observed max in-flight handlers ≥ 2 — serial
  dispatch can never exceed 1), and an `#[ignore]`d CPU-bound benchmark
  `sharded_web_server_cpu_bound_throughput_scales` demonstrates wall-clock
  scaling (~5.3× on 14 cores for a busy-loop handler). See
  `code/specs/WEB01-async-web-core.md`.

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
