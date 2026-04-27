# Changelog

## [0.1.0] - 2026-04-12

### Added
- `Framer` class with `feed()`, `frames()`, `reset()`, `buffer_size`
- bytearray-based internal buffer with O(1) append
- LF-only client tolerance (strips `\r\n` or `\n`)
- 510-byte content limit enforcement per RFC 1459
- Full test suite with 95%+ coverage
