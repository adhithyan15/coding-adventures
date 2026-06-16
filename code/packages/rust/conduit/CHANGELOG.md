# Changelog

All notable changes to this package will be documented in this file.

## [0.3.0] - 2026-06-16

### Added

- **`MailboxServer` (WEB01b-2)** — an opt-in, per-request-parallel server, the
  mailbox counterpart of `ShardedServer`. Where `ShardedServer` parallelises *by
  connection* (N reactor shards), `MailboxServer::bind(host, port, worker_count,
  app)` parallelises *by request*: a single reactor frames each request and
  submits it to a `worker_count`-thread pool (via `web_core::MailboxWebServer`),
  so a worker runs the Conduit dispatch off the reactor thread. Routing, hooks,
  and `halt` behave exactly as with `Server`; the application is shared (`Arc`)
  across pool threads unchanged. Surface: `bind` / `bind_with_options` /
  `local_addr` / `is_running` / `stop` / `serve` (and `Clone`). Binding is a
  single cross-platform call returning `std::io::Result` (the mailbox stack is
  `io::Error`-based), unlike the per-platform `Server` / `ShardedServer` binds.
  Correct and in order for one-request-and-close and *sequential* keep-alive;
  pipelined-connection gating/reordering is WEB01b-1b. `Server` (single reactor)
  remains the default. See `code/specs/WEB01b-mailbox-parallelism.md`.
- Test `mailbox_server_dispatches_handlers_concurrently` deterministically proves
  pool parallelism on a single reactor through the full facade (observed max
  in-flight handlers >= 2 — inline dispatch never exceeds 1).

## [0.2.0] - 2026-06-16

### Added

- **`ShardedServer` (WEB01a-3)** — an opt-in parallel server. Where `Server`
  runs every connection on a single reactor thread, `ShardedServer::bind(host,
  port, worker_count, app)` dispatches handlers across `worker_count` reactor
  shards (via `web_core::ShardedWebServer`), so requests on different connections
  are handled concurrently and a slow/CPU-bound handler no longer stalls the
  whole server. Mirrors `Server`'s surface (`bind` / `bind_with_options` /
  `serve` / `local_addr` / `stop_handle`) plus `worker_count`. Routing, hooks,
  `halt`, and HTTP/1.1 per-connection response ordering are unchanged — only the
  degree of concurrency differs. `Server` (single reactor) remains the default;
  parallelism is opt-in. See `code/specs/WEB01-async-web-core.md`.
- Test `sharded_server_dispatches_handlers_concurrently` deterministically proves
  cross-shard parallelism through the full facade (observed max in-flight
  handlers >= 2 — a single reactor can never exceed 1).

## [0.1.0] - 2026-05-05

### Added

- Rust-native `Application` facade over `web-core::WebApp`.
- Route registration helpers for `GET`, `POST`, `PUT`, `DELETE`, `PATCH`, and
  arbitrary HTTP methods.
- Application settings and route introspection.
- Before filters, observing after filters, response-transforming after hooks,
  custom not-found handlers, custom method-not-allowed handlers, and panic
  recovery hooks.
- `Server` wrapper that binds to the native platform backend through
  `web-core::WebServer`.
- Response helpers: `text`, `html`, `json`, `redirect`, `halt`, and
  explicit-status variants.
- `RequestExt` helpers for route params, query params, and body text.
- Tests for routing, hooks, response helpers, request helpers, JSON escaping, and
  real TCP serving on a platform-native backend.
