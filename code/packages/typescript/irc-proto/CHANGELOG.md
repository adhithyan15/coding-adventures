# Changelog — @coding-adventures/irc-proto

All notable changes to this package will be documented here.

## [0.1.0] — 2026-04-12

### Added

- Initial TypeScript port of the Python `irc-proto` package
- `Message` interface: `{ prefix: string | null, command: string, params: string[] }`
- `ParseError` class extending `Error` with `name = "ParseError"`
- `parse(line: string): Message` — 3-stage RFC 1459 parser (prefix, command, params)
- `serialize(msg: Message): Buffer` — CRLF-terminated Buffer output
- RFC 1459 compliance: uppercase normalization, max 15 params, trailing colon logic
- Comprehensive test suite: 37 tests, 97%+ coverage
- Literate inline documentation explaining the IRC message grammar
