# Changelog
All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-12
### Added
- Initial package scaffolding
- `TcpError` enum with 7 structured error variants (DNS, refused, timeout, reset, pipe, EOF, IO)
- `ConnectOptions` struct with configurable connect/read/write timeouts and buffer size
- `TcpConnection` class with buffered I/O over POSIX sockets
  - `readLine()` — read until newline, returns String
  - `readExact(_:)` — read exactly N bytes, returns Data
  - `readUntil(_:)` — read until delimiter byte, returns Data
  - `writeAll(_:)` — write all bytes via send() loop
  - `flush()` — no-op (direct send), maintains API contract
  - `shutdownWrite()` — TCP half-close via shutdown(SHUT_WR)
  - `peerAddr()` / `localAddr()` — address queries via getpeername/getsockname
  - `close()` — idempotent close with deinit safety
- `tcpConnect(host:port:options:)` — DNS resolution via getaddrinfo, non-blocking connect with select() timeout, multi-address iteration (Happy Eyeballs simplified)
- Portable fd_set helpers for select() (Darwin + Glibc)
- errno-to-TcpError mapping for POSIX error codes
- 22 XCTest cases using POSIX echo/silent/partial/request-response/half-close server helpers
