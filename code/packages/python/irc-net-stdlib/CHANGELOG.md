# Changelog

## [0.1.0] - 2026-04-12

### Added
- `Connection`, `Listener`, `Handler`, `EventLoop` stable Protocol interfaces
- `StdlibConnection` — wraps socket.socket with typed read/write/close
- `StdlibListener` — bound TCP listener with SO_REUSEADDR
- `StdlibEventLoop` — thread-per-connection event loop with dual-lock threading model
- `create_listener(host, port)` factory function
- Integration tests using real TCP sockets (no mocks)
