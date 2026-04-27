# Changelog — irc_framing (Elixir)

## [0.1.0] — 2026-04-12

### Added

- `CodingAdventures.IrcFraming.Framer` struct with `:buf` field (binary buffer)
- `Framer.new/0` — create a fresh framer with an empty buffer
- `Framer.feed/2` — append raw bytes to the buffer; returns updated framer
- `Framer.frames/1` — extract all complete CRLF-terminated lines from the buffer;
  returns `{new_framer, [String.t()]}` where lines have the CRLF suffix stripped
- `Framer.reset/1` — clear the buffer (useful after error recovery)
- `Framer.buffer/1` — inspect the current buffered bytes (for diagnostics)
- `CodingAdventures.IrcFraming` facade module delegating all `Framer` functions
- Handles both `\r\n` (CRLF) and bare `\n` (LF) line endings
- Silently discards lines exceeding 510 bytes (RFC 1459 maximum line length)
- Comprehensive ExUnit test suite — 30 tests, 100% coverage
- Pure functional design — no side effects, no processes
- Port of Python reference implementation to idiomatic Elixir
