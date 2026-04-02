# Changelog

## 0.1.1 — 2026-03-31

### Changed

- Wrapped public functions `ParseCSV` and `ParseCSVWithDelimiter` with the
  Operations system (`StartNew[T]`) for automatic timing, structured logging,
  and panic recovery. Public API signatures are unchanged.

## 0.1.0 — 2026-03-25

Initial release.

### Added

- `ParseCSV(source string) ([]map[string]string, error)` — parse CSV with default
  comma delimiter
- `ParseCSVWithDelimiter(source string, delimiter rune) ([]map[string]string, error)` —
  parse CSV with configurable delimiter
- `CsvError` type implementing the `error` interface for parse errors
- Four-state character-by-character state machine:
  - `StateFieldStart` — decide quoted vs. unquoted
  - `StateInUnquotedField` — collect plain field characters
  - `StateInQuotedField` — collect quoted field characters (only `"` is special)
  - `StateInQuotedMaybeEnd` — resolve `""` escape vs. end-of-field
- `parser` struct encapsulating all mutable parser state, driven by `step(rune, bool)`
- Quoted fields support embedded commas, embedded newlines, and `""` escape sequences
- Configurable delimiter (comma, tab, semicolon, pipe, any rune)
- Ragged row handling: short rows padded with `""`, long rows truncated
- All three newline conventions: `\n` (Unix), `\r\n` (Windows), `\r` (old Mac)
- Tolerant handling of malformed post-quote characters (re-process from FIELD_START)
- Empty file → nil; header-only file → nil
- Returns `*CsvError` for unclosed quoted fields
- Comprehensive test suite: 50 test cases, 98.1% statement coverage
- Literate-programming style source with state-machine diagram, truth tables, and
  inline explanations throughout
