# Changelog — csv-parser (Rust)

## [0.1.0] — 2026-03-25

### Added

- Initial implementation of `parse_csv(source: &str)` — parses CSV using the default
  comma delimiter and returns `Result<Vec<HashMap<String, String>>, CsvError>`.
- `parse_csv_with_delimiter(source: &str, delimiter: char)` — same as above but with
  a configurable single-character delimiter (e.g., `'\t'` for TSV, `';'` for European CSV).
- Four-state machine: `FieldStart`, `InUnquotedField`, `InQuotedField`, `InQuotedMaybeEnd`.
- Full support for RFC 4180 quoted fields: commas, newlines, and `""` escape sequences
  inside quoted fields are handled correctly.
- Ragged row handling: short rows padded with `""`, long rows truncated to header length.
- Support for all three newline styles: `\n` (Unix), `\r\n` (Windows), `\r` (old Mac).
- `CsvError::UnclosedQuote` error variant returned when EOF is reached inside a quoted field.
- Comprehensive unit test suite covering: basic tables, quoted fields with commas/newlines,
  escaped quotes, empty fields, ragged rows, empty input, header-only input, Windows/Mac
  line endings, custom delimiters, whitespace preservation.
- No external dependencies — pure Rust standard library only.
- Literate programming style: every function, state, and edge case documented inline.
