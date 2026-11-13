# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for the sibling libraries) instead of an `echo ... skipping` recipe. The suite uses loopback sockets and `fork()` (emulated with threads on Windows), so the tests now stay inside the subset both platforms support.

### Fixed

- `t/01-basic.t`: the read-timeout server child now blocks until the client hangs up instead of `sleep(10)` followed by `kill('TERM')` (a pseudo-process cannot be interrupted out of `accept`/`sleep` on Windows); the short pause uses `Time::HiRes::sleep` instead of a three-`undef` 4-argument `select`; the connect-timeout test also accepts `connection_refused`, which is what an egress proxy or sandbox firewall answers for a non-routable address.
- `t/01-basic.t` `connection refused`: retries on another port if the connect unexpectedly succeeds (a connect to a just-freed ephemeral port can be given that same port as its own source port and connect to itself) and prints what happened if it still fails; the macOS leg of the Windows-recipe PR failed this subtest with no library change in the diff.
- `connect` verifies the connection really exists: with a `Timeout`, `IO::Socket::INET` connects non-blocking and select()s for writability, a refused connect is also "writable", and on macOS the constructor handed back a socket with the error still pending (the macOS CI leg saw `connect` to a closed loopback port return a connection on all five retries). It now checks `getpeername` and `SO_ERROR` and throws `connection_refused` / `timeout` / `io_error`. New test `unconnected socket from the constructor is rejected` simulates this on every OS.

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
