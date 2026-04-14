# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- Initial implementation ported from Rust tcp-client crate
- `connect/3` with configurable timeouts and buffer size
- `read_line/1` for line-oriented protocol parsing
- `read_exact/2` for fixed-length reads (e.g., HTTP Content-Length)
- `read_until/2` for delimiter-based reads (e.g., null-terminated strings)
- `write_all/2` for sending data
- `flush/1` (no-op, API parity with Rust)
- `shutdown_write/1` for TCP half-close
- `peer_addr/1` and `local_addr/1` for address inspection
- `close/1` for connection teardown
- Error mapping from Erlang atoms to domain-specific atoms
- 26 ExUnit tests covering echo, timeout, error, EOF, and address scenarios
- Literate programming style with inline explanations and diagrams
