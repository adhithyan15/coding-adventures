# Changelog

All notable changes to this package will be documented in this file.

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
