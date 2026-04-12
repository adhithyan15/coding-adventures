# Changelog

All notable changes to `irc-proto` (Go) will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- Initial Go implementation ported from the Python reference implementation.
- `Message` struct with `Prefix`, `Command`, and `Params` fields.
- `ParseError` type returned on malformed input.
- `Parse(line string) (*Message, error)` — parses a CRLF-stripped IRC line.
  - Strips optional `:prefix` prefix.
  - Uppercases the command token.
  - Collects up to 15 params; `:trailing` param absorbs the rest of the line.
  - Returns `*ParseError` for empty/whitespace-only or prefix-only input.
- `Serialize(msg *Message) []byte` — serialises back to CRLF-terminated wire bytes.
  - Adds `:prefix` when `Prefix` is non-empty.
  - Wraps the last param in `:` when it contains spaces.
- 18 unit tests covering all parsing edge cases and round-trip fidelity.
- 97.7% statement coverage.
