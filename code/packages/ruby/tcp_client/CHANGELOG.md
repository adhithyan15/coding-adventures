# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- `TcpClient.connect(host, port, options)` establishes TCP connections with configurable timeouts
- `TcpConnection` class with buffered I/O: `read_line`, `read_exact`, `read_until`, `write_all`, `flush`
- `ConnectOptions` with `connect_timeout`, `read_timeout`, `write_timeout`, `buffer_size` (defaults: 30s, 30s, 30s, 8192)
- `shutdown_write` for TCP half-close (client signals end of sending)
- `peer_addr` and `local_addr` return `[host, port]` pairs
- Structured error hierarchy: `TcpError` base with `DnsResolutionFailed`, `ConnectionRefused`, `Timeout`, `ConnectionReset`, `BrokenPipe`, `UnexpectedEof`
- Timeout enforcement via `IO.select` for both reads and writes
- 23 tests covering echo round-trips, timeouts, error mapping, half-close, and edge cases
- Literate programming style with inline explanations, diagrams, and truth tables
