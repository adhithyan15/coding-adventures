# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-12

### Added
- Full TCP client implementation with OOP API
- `CodingAdventures::TcpClient::ConnectOptions` configuration class
  - Builder pattern with chainable setters
  - Defaults: connect_timeout=30s, read_timeout=30s, write_timeout=30s, buffer_size=8192
- `CodingAdventures::TcpClient::TcpConnection` connection class
  - `read_line()` for line-oriented protocol reading (until \n)
  - `read_exact($n)` for fixed-size binary reads with looping
  - `read_until($delimiter)` for delimiter-terminated reads (delimiter as integer 0-255)
  - `write_all($data)` with syswrite loop for complete delivery
  - `flush()` for explicit buffer flushing
  - `shutdown_write()` for TCP half-close (SHUT_WR)
  - `peer_addr()` and `local_addr()` returning "host:port" strings
  - `close()` with safe double-close behavior
  - Automatic cleanup via DESTROY (Perl's Drop equivalent)
- `CodingAdventures::TcpClient::TcpError` structured error class
  - Error types: dns_resolution_failed, connection_refused, timeout, connection_reset, broken_pipe, unexpected_eof, io_error
  - Stringification overload for readable die messages
  - `type()` and `message()` accessors for programmatic matching
- `connect($host, $port, $options)` free function
  - Accepts ConnectOptions object, hashref, or undef for defaults
  - Error classification from IO::Socket::INET error strings
- Internal buffered reading via sysread() to avoid Perl IO buffer / IO::Select conflicts
- 27 test cases covering echo server, read modes, timeouts, errors, half-close, edge cases
- Knuth-style literate programming with extensive inline documentation
