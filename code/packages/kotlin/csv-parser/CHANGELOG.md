# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-25

### Added

- `parseCSV(String)` — top-level function that parses comma-delimited CSV,
  returning `List<Map<String, String>>`.
- `parseCSVWithDelimiter(String, Char)` — like `parseCSV` but with a
  configurable delimiter.
- `CsvParseException` (unchecked in Kotlin) thrown on unclosed quoted fields.
- `ParseState` enum with four states: `FIELD_START`, `IN_UNQUOTED_FIELD`,
  `IN_QUOTED_FIELD`, `IN_QUOTED_MAYBE_END`.
- Hand-rolled character-by-character state machine in private `Parser` class;
  no dependency on any standard library CSV facilities.
- RFC 4180 quoted fields: commas, embedded newlines, and `""` escaped quotes
  inside quoted fields all handled correctly.
- Ragged row handling: short rows padded with `""`, long rows truncated to
  header length.
- Newline variant support: `\n` (Unix), `\r\n` (Windows), `\r` (old Mac).
- Blank line skipping: blank lines between data rows are silently ignored.
- Trailing newline tolerance: files with or without trailing newline both parse
  correctly.
- Map key ordering: output maps use `buildMap` (backed by `LinkedHashMap`) to
  preserve column insertion order from the header.
- 29 Kotlin/JUnit 5 tests covering: basic CSV, quoted fields, ragged rows,
  custom delimiter, newline variants, edge cases, error cases, and whitespace.
