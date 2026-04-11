# Changelog

All notable changes to `coding-adventures-tcp-server` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-04-08

### Added

- `TcpServer` class: single-threaded TCP server using `selectors.DefaultSelector`
  for OS-level I/O multiplexing (epoll/kqueue/select depending on platform).
- `Handler` type alias: `Callable[[bytes], bytes]` — the pluggable handler API.
- `TcpServer.start()`: binds and listens without blocking.
- `TcpServer.serve()`: enters the select-based event loop; blocks until `stop()`.
- `TcpServer.serve_forever()`: convenience wrapper: `start()` then `serve()`.
- `TcpServer.stop()`: thread-safe shutdown signal via `threading.Event`.
- `TcpServer.address` property: returns `(host, port)` after `start()`.
- `TcpServer.is_running` property: `True` while the event loop is active.
- Context manager support (`__enter__` / `__exit__`).
- `__repr__` showing host, port, and running status.
- Default echo handler when `handler=None` is passed.
- `SO_REUSEADDR` socket option to avoid `Address already in use` on restart.
- Non-blocking sockets throughout (`setblocking(False)`) for correct event loop behaviour.
- `_cleanup()`: closes all registered fds and resets the selector on shutdown.
- Full test suite in `tests/test_tcp_server.py` with >95% coverage.
- `py.typed` marker for PEP 561 compliance.
- `README.md` with quick-start examples, API reference, and syscall table.

### Notes

- This is the DT24 layer: pure I/O, no protocol knowledge. RESP framing
  and Redis command dispatch are the responsibility of the DT25 (mini-redis)
  layer.
- The `selectors.DefaultSelector` approach (epoll/kqueue) is how Redis itself
  achieves >1M ops/sec on a single thread.
- Partial reads: each `recv()` call invokes the handler with however many bytes
  arrived. Callers that need complete RESP messages must buffer at the DT25
  layer (see spec section "The Partial Read / Write Problem").
