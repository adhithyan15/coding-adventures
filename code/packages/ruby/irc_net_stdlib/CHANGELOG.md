# Changelog

## [0.1.0] - 2026-04-12

### Added
- Initial implementation: `StdlibEventLoop` with `run`, `stop`, `send_to`
- `StdlibConnection` with thread-safe `write` and `close`
- `Handler` mixin with default no-op callbacks
- `alloc_conn_id` module-level function
