# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- `connect()` function with DNS resolution and configurable connect timeout
- `TcpConnection` struct with buffered reader/writer (`BufReader`/`BufWriter`)
- `read_line()`, `read_exact()`, `read_until()` for buffered reading
- `write_all()` and `flush()` for buffered writing
- `shutdown_write()` for TCP half-close
- `peer_addr()` and `local_addr()` for connection endpoints
- `ConnectOptions` with connect/read/write timeouts and buffer size
- `TcpError` enum with 7 variants (DnsResolutionFailed, ConnectionRefused, Timeout, ConnectionReset, BrokenPipe, UnexpectedEof, IoError)
- 21 unit tests + 2 doc-tests covering echo server, timeouts, errors, half-close, and edge cases
