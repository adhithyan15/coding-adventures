# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- `connect(host, port, options)` function to establish TCP connections
- `TcpConnection` class with buffered reading via `socket.makefile('rb')`
  - `read_line()` for line-oriented protocols (HTTP, SMTP, RESP)
  - `read_exact(n)` for fixed-length reads (Content-Length)
  - `read_until(delimiter)` for custom delimiters (null-terminated, RESP)
  - `write_all(data)` using `socket.sendall()` for reliable writes
  - `flush()` no-op for API compatibility
  - `shutdown_write()` for half-close (signals EOF to server)
  - `peer_addr()` and `local_addr()` for connection introspection
  - `close()` with idempotent cleanup
  - Context manager support (`with connect(...) as conn:`)
- `ConnectOptions` for configuring timeouts and buffer size
- Structured error hierarchy:
  - `TcpError` (base)
  - `DnsResolutionFailed` with host and message attributes
  - `ConnectionRefused` with addr attribute
  - `Timeout` with phase and duration attributes
  - `ConnectionReset`
  - `BrokenPipe`
  - `UnexpectedEof` with expected and received attributes
- 41 tests covering echo server, timeouts, errors, half-close, edge cases
- 92% test coverage
- Full type annotations passing mypy --strict
- Literate programming style with inline documentation
