# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] - 2026-11-14

### Added

- **A native Windows engine.** `TcpReactor` now builds and serves on Windows
  through an I/O completion port and Winsock overlapped I/O (`AcceptEx`,
  `WSARecv`, `WSASend`), using the `iocp` crate. It is a completion engine, not
  an `EventBackend`: on Windows `TcpReactor` is not generic over a backend, and
  has `bind_iocp` and `with_listener`. Buffer lifetime follows the overlapped-I/O
  rule that the kernel owns a buffer until its completion is dequeued, so a
  closing connection stays in the table until its operations have completed, and
  anything still outstanding after a drain deadline is leaked rather than freed.
- `TcpReactor::bind_epoll` on Linux.
- `NativeTcpReactor` and `bind_native` so portable code and tests pick the native
  engine without `cfg`.
- `tests/reactor.rs`: eight behaviour tests written against the public API that run
  on Linux, macOS and Windows (concurrent echo, peer/local addresses, connection
  cap, pending-write budget, a large response to a slow reader, many requests on one
  connection, stop and serve again, dropping reactors). Windows-only unit tests
  cover the completion engine's bookkeeping.
- Real `BUILD_windows`; the crate is no longer `BUILD_TOOL_UNSUPPORTED` there.

### Changed

- The readiness engine moved, unchanged, into `src/readiness.rs`; `lib.rs` holds the
  shared types. The `compile_error!` for non-Unix targets is gone.
- The three macOS/BSD-only tests that poked reactor internals were replaced by the
  cross-platform public-API tests above, which cover the same behaviour. On Linux
  the crate previously ran no tests at all.

## [0.1.0] - 2026-04-17

### Added

- `TcpReactor` generic over `native-event-core::EventBackend`
- `StopHandle` for cooperative shutdown
- macOS/BSD `bind_kqueue` convenience constructor
- end-to-end concurrent echo test on top of `kqueue`
- configurable limits for active connections and per-connection queued writes
- tests covering connection-cap rejection and write-budget overflow shutdown

### Fixed

- stabilized socket-cap tests so they rely on bounded retries and reactor state
  instead of immediate loopback close propagation
