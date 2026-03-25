# Changelog

All notable changes to `coding_adventures_csv_parser` are documented here.

## [0.1.0] — 2026-03-25

### Added

- Initial implementation of the CSV parser as a hand-rolled state machine.
- `CodingAdventures::CsvParser.parse_csv(source)` — parses CSV text into an array of `Hash{String => String}` row maps.
- `CodingAdventures::CsvParser.parse_csv(source, delimiter: ",")` — supports a configurable field delimiter (e.g., `"\t"` for TSV, `";"` for European CSV).
- Four-state machine: `FIELD_START`, `IN_UNQUOTED_FIELD`, `IN_QUOTED_FIELD`, `IN_QUOTED_MAYBE_END`.
- RFC 4180-compatible quoted fields: embedded commas, newlines, and escaped double-quotes (`""`) all supported.
- Ragged row handling: short rows are padded with `""`, long rows are truncated to header length.
- `CodingAdventures::CsvParser::UnclosedQuoteError` raised when a quoted field is never closed before EOF.
- All values returned as strings — no type coercion.
- Comprehensive minitest test suite with >95% coverage via SimpleCov.
- Literate-programming inline documentation throughout.
