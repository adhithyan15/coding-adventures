# Changelog

## [0.1.0] - 2026-04-12

### Added
- `Message` dataclass with `prefix`, `command`, `params` fields
- `parse()` function — RFC 1459 message parsing with 15-param cap
- `serialize()` function — CRLF-terminated wire format output
- `ParseError` exception for malformed input
- Full test suite with 95%+ coverage
