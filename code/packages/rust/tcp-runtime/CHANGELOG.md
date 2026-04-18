# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-18

### Added

- `TcpRuntime` as the first TCP-specific runtime facade above `stream-reactor`
- `TcpRuntimeOptions` for listener policy, stream policy, and runtime limits
- `TcpConnectionInfo` with both peer and local listener addresses
- `TcpHandlerResult` for queued writes plus close-after-flush intent
- host-OS convenience constructors for `kqueue`, `epoll`, and Windows transport
  providers
- macOS / BSD end-to-end tests for echo behavior, local-address metadata,
  connection caps, queued-write overflow, and stop-handle shutdown

## [0.1.1] - 2026-04-18

### Added

- `bind_with_state` plus stateful OS convenience constructors for protocol
  session state
- close callbacks that observe final TCP connection state during teardown
- tests proving stateful handlers preserve per-connection state across multiple
  reads
