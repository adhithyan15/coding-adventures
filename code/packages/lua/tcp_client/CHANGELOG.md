# Changelog
All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-12
### Added
- `M.connect(host, port, options)` -- DNS resolution + TCP connect with configurable timeout, returns TcpConnection
- `M.ConnectOptions.new(opts)` -- configuration table with connect_timeout (30s), read_timeout (30s), write_timeout (30s), buffer_size (8192)
- `M.TcpError` -- structured error type with `type` field (dns_resolution_failed, connection_refused, timeout, connection_reset, broken_pipe, unexpected_eof, io_error)
- `TcpConnection:read_line()` -- buffered read until `\n`, returns line including delimiter, empty string at EOF
- `TcpConnection:read_exact(n)` -- read exactly n bytes, returns unexpected_eof error if connection closes early
- `TcpConnection:read_until(delimiter)` -- read until delimiter string or byte value, returns data including delimiter
- `TcpConnection:write_all(data)` -- write all bytes with automatic retry on partial sends
- `TcpConnection:flush()` -- no-op for API compatibility (luasocket sends immediately)
- `TcpConnection:shutdown_write()` -- half-close the write direction, read half stays open
- `TcpConnection:peer_addr()` -- returns remote address as "ip:port" string
- `TcpConnection:local_addr()` -- returns local address as "ip:port" string
- `TcpConnection:close()` -- close the connection and release the socket
- Internal read buffer for efficient read_line/read_exact/read_until without byte-at-a-time syscalls
- Comprehensive test suite (25+ tests) covering echo server, timeouts, errors, half-close, and edge cases
