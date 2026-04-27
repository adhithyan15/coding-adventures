# Changelog — irc_proto (Elixir)

## [0.1.0] — 2026-04-12

### Added

- `CodingAdventures.IrcProto.Message` struct with `:prefix`, `:command`, `:params` fields
- `CodingAdventures.IrcProto.parse/1` — parse a raw IRC line into a `Message`
  - Handles optional `:prefix` header
  - Normalises command to uppercase
  - Supports up to 15 params with trailing (`:`) param
  - Returns `{:ok, Message.t()}` or `{:error, String.t()}`
- `CodingAdventures.IrcProto.serialize/1` — render a `Message` back to a wire string with CRLF
- `CodingAdventures.IrcProto` facade module with `parse/1` and `serialize/1`
- Comprehensive ExUnit test suite — 27 tests, 95.12% coverage
- Pure functional design — no side effects, no processes
- Port of Python reference implementation to idiomatic Elixir
