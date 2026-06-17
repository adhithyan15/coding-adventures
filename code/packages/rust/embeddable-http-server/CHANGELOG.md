# Changelog

## Unreleased

- **WEB01b-1b: pipelined-response ordering for `MailboxHttpServer`.** The server
  now enables the mailbox's `ordered_responses`, so a single connection's replies
  are written back in **request order** even when the worker pool finishes them
  out of order — full HTTP/1.1 keep-alive pipelining is now correct (1a already
  submitted every framed request; 1b makes the wire order right). The per-
  connection reorder buffer is bounded by the pool's queue depth, so a connection
  that pipelines past it is shed with a 503 rather than buffering unboundedly. New
  test `mailbox_http_server_preserves_pipelined_response_order` pipelines 4
  requests whose handlers finish in reverse order and asserts the bodies come back
  `r0, r1, r2, r3` (it fails without the reorder buffer). See
  `code/specs/WEB01b-mailbox-parallelism.md`.

- **WEB01b-1a: `MailboxHttpServer`** — a deferred-response, *per-request*-parallel
  HTTP server. A single reactor frames each request and **submits it as a job** to
  an in-process `worker_count`-thread pool (`embeddable-tcp-server`'s
  `new_inprocess_mailbox`); a worker runs the handler and the pool's response
  router writes the serialized response back to the originating connection. This
  decouples handler concurrency from the I/O thread (unlike `ShardedHttpServer`,
  which is parallel *by connection*). The platform (kqueue/epoll/IOCP) is selected
  internally, so it is cross-platform with no per-OS binds. Scope (WEB01b-1a):
  each framed request is submitted to the pool as it arrives and the router
  writes responses back as workers finish — correct and in order for
  one-request-and-close and *sequential* keep-alive (at most one request in
  flight, so the unordered pool cannot reorder). Gating a *pipelined* connection
  to one in-flight request and reordering the pool's responses into HTTP/1.1 wire
  order needs a per-connection reorder buffer and is WEB01b-1b. (We deliberately
  do **not** use `stream-reactor`'s `defer_read`: it *replays* the deferred chunk
  on resume, which corrupts framing for bytes the handler already consumed —
  re-feeding a TCP-fragmented tail caused a spurious `400` before the real
  response under load.) Pool-queue-full sheds load with a 503 (backpressure). New
  test `mailbox_http_server_handles_requests_concurrently` deterministically
  proves single-reactor pool parallelism (observed max in-flight handlers >= 2 —
  inline dispatch never exceeds 1). See `code/specs/WEB01b-mailbox-parallelism.md`.
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
