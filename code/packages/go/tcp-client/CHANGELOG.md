# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- `Connect()` function with DNS resolution, configurable timeouts, and buffered I/O
- `TcpConnection` struct with `ReadLine`, `ReadExact`, `ReadUntil`, `WriteAll`, `Flush`
- `ShutdownWrite()` for TCP half-close (FIN)
- `PeerAddr()` and `LocalAddr()` for address inspection
- `Close()` for connection teardown
- `ConnectOptions` struct with `DefaultOptions()` factory
- `TcpError` with structured error kinds: DnsResolutionFailed, ConnectionRefused, Timeout, ConnectionReset, BrokenPipe, UnexpectedEof, IoError
- Error mapping from Go's `net`/`syscall` errors to `TcpError` variants
- 35 tests covering echo round-trips, timeouts, error conditions, half-close, and edge cases (83% coverage)
- Literate programming style with inline explanations and ASCII diagrams
