# Changelog

All notable changes to `irc-net-stdlib` (Go) will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- Initial Go implementation of the IRC TCP event loop.
- `ConnID` type (`int64`) for opaque connection identifiers.
- `Handler` interface: `OnConnect`, `OnData`, `OnDisconnect`.
- `EventLoop` struct with goroutine-per-connection model.
- `NewEventLoop()` constructor.
- `Run(addr string, handler Handler) error` — starts TCP listener; blocks until stopped.
- `Stop()` — closes listener, drains all active connections.
- `SendTo(connID ConnID, data []byte)` — thread-safe write to a single connection.
- Separate mutexes for handler serialisation (`handlerMu`) and connection map
  access (`connsMu`) to prevent deadlocks while allowing concurrent sends.
- 96%+ statement coverage across unit tests.
