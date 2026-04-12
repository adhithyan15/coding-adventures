# Changelog — @coding-adventures/irc-framing

All notable changes to this package will be documented here.

## [0.1.0] — 2026-04-12

### Added

- Initial TypeScript port of the Python `irc-framing` package
- `Framer` class with `feed(data: Buffer)`, `frames(): Buffer[]`, `reset()`, and `bufferSize` getter
- RFC 1459 compliance: 510-byte content limit, CRLF and bare LF support
- Comprehensive test suite covering partial data, multi-message feeds, overlong lines, and buffer management
- Literate inline documentation explaining the TCP framing problem
