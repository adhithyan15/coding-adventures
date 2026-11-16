# Changelog — irc_framing (Elixir)

## [Unreleased]

### Fixed

- SEC-011 (CWE-400/770): `Framer.feed/2` buffered input without limit when no LF arrived, and the moduledoc wrongly claimed the 510-byte rule prevented memory exhaustion (it only applies to lines that already ended in LF). The unterminated tail is now capped at 511 bytes (510 content plus a CR; RFC 1459 and RFC 2812 section 2.3: 512 including CRLF). Past the cap the partial line is dropped and input is skipped up to the next LF (fail closed, same silent discard as a complete overlong line; no function signature changes, so no version bump). The struct gains `:tail_len` and `:discarding` fields (defaults `0` / `false`; `:buf` is still the only enforced key). `reset/1` also clears the discard state. Ported from the Rust fix (PR #19359).
- Moduledoc corrected to state the real memory bound.

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
