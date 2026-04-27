# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-25

### Added

- `CsvParser` utility class with two public static methods:
  - `parseCSV(String source)` — parses comma-delimited CSV
  - `parseCSVWithDelimiter(String source, char delimiter)` — configurable
    delimiter (tab, semicolon, pipe, etc.)
- Both methods return `List<Map<String, String>>` with header-keyed rows.
  Maps use `LinkedHashMap` to preserve column insertion order.
- `CsvParseException` (checked exception) thrown on unclosed quoted fields.
- `ParseState` enum with four states: `FIELD_START`, `IN_UNQUOTED_FIELD`,
  `IN_QUOTED_FIELD`, `IN_QUOTED_MAYBE_END`.
- Hand-rolled character-by-character state machine in private `Parser` inner
  class; no dependency on `java.io.BufferedReader`, regex, or external libs.
- RFC 4180 quoted fields: commas, embedded newlines, and `""` escaped quotes
  inside quoted fields all handled correctly.
- Ragged row handling: short rows padded with `""`, long rows truncated to
  header length.
- Newline variant support: `\n` (Unix), `\r\n` (Windows), `\r` (old Mac).
- Blank line skipping: blank lines between data rows are silently ignored.
- Trailing newline tolerance: files with or without a trailing newline both
  parse correctly.
- Tolerant handling of `"other"` after closing quote in `IN_QUOTED_MAYBE_END`:
  field is ended and the unknown character is re-processed.
- 29 JUnit 5 tests covering: basic CSV, quoted fields, ragged rows, custom
  delimiter, newline variants, edge cases, error cases, and whitespace handling.
