# Changelog

## 0.1.0 — 2026-03-25

Initial release.

### Added

- `CsvParser.parse_csv/1` — parse CSV text with default comma delimiter; returns
  `{:ok, rows}` or `{:error, reason}`
- `CsvParser.parse_csv/2` — same with configurable single-character delimiter
- Four-state character-by-character state machine:
  - `FIELD_START` — decide quoted vs. unquoted
  - `IN_UNQUOTED_FIELD` — collect plain field characters
  - `IN_QUOTED_FIELD` — collect quoted field characters (only `"` is special)
  - `IN_QUOTED_MAYBE_END` — resolve `""` escape vs. end-of-field
- Quoted fields support embedded commas, embedded newlines, and `""` escape sequences
- Configurable delimiter (comma, tab, semicolon, pipe, etc.)
- Ragged row handling: short rows padded with `""`, long rows truncated
- All three newline conventions: `\n` (Unix), `\r\n` (Windows), `\r` (old Mac)
- Empty file → `{:ok, []}`; header-only file → `{:ok, []}`
- Returns `{:error, "Unclosed quoted field at end of input"}` for malformed input
- Comprehensive test suite (95%+ coverage): 40+ test cases covering all edge cases
- Literate-programming style source with state-machine diagram, truth tables,
  and inline explanations throughout
