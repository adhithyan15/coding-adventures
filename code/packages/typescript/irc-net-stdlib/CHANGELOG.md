# Changelog — @coding-adventures/irc-net-stdlib

All notable changes to this package will be documented here.

## [0.1.0] — 2026-04-12

### Added

- Initial TypeScript port of the Python `irc-net-stdlib` package (event-driven instead of thread-per-connection)
- `ConnId` branded type for connection identity
- `Handler` interface: `onConnect()`, `onData()`, `onDisconnect()` callbacks
- `EventLoop` class using Node.js `net` module:
  - `run(host, port, handler): Promise<void>` — starts listening, resolves on `stop()`
  - `stop()` — closes server and all active sockets
  - `sendTo(connId, data)` — write bytes to a specific connection
  - `listenPort` getter — returns actual bound port (useful with port=0 in tests)
- Integration test suite using real TCP sockets on ephemeral ports
- Literate inline documentation explaining the event-driven model vs. thread-per-connection
