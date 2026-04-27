# Changelog — irc-net-stdlib

All notable changes to this crate will be documented here.

## [0.1.0] — 2026-04-12

### Added

- `ConnId(u64)` newtype for connection identifiers
- `Handler` trait with `on_connect`, `on_data`, `on_disconnect` methods
- `EventLoop` struct with:
  - `new()` constructor
  - `run(addr, handler) -> io::Result<()>` — non-blocking accept loop, thread-per-connection
  - `stop()` — sets `running = false` to terminate the accept loop
  - `send_to(conn_id, data)` — writes bytes to a connection (safe no-op if not found)
- Thread-per-connection worker: blocking read loop with `handler_lock` serialization
- `Default` trait implementation
- Integration tests using real TCP connections:
  - `test_connect_data_disconnect` — lifecycle events recorded in order
  - `test_send_to_delivers_data` — server-to-client writes via `send_to()`
  - `test_stop_terminates_run` — `stop()` exits the accept loop
  - `test_multiple_connections` — 3 simultaneous connections tracked independently
