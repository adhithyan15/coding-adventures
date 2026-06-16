# Changelog

## Unreleased

- **WEB01a-1: `ShardedHttpServer`** — a parallel counterpart to `HttpServer`.
  It runs the same per-connection `HttpConnectionState` machine across
  `worker_count` reactor threads (a `ShardedTcpRuntime`), so requests on
  different connections are handled concurrently across shards. The request
  handler contract is unchanged (still a synchronous, `Send + Sync`
  `Fn(HttpRequest) -> HttpResponse` invoked inline on the owning shard), so
  HTTP/1.1 per-connection response ordering is preserved. Platform
  constructors: `bind_kqueue_sharded` (macOS/BSD), `bind_epoll_sharded`
  (Linux), `bind_windows_sharded` (Windows). Existing `HttpServer` is
  untouched — parallelism is opt-in via this new type. New test
  `sharded_http_server_serves_concurrent_clients_across_shards` proves 16
  concurrent clients × 4 requests are each handled exactly once across 4
  shards. See `code/specs/WEB01-async-web-core.md`.
- Add an ignored native-server stress test for concurrent HTTP/1 pipelined
  requests over real sockets.

## 0.1.0

- Add the initial HTTP/1 embeddable server primitive.
