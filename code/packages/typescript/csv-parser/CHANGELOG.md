# Changelog — csv-parser (TypeScript)

## [0.1.0] — 2026-03-25

### Added

- Initial implementation of `parseCSV(source: string)` — parses CSV using the default
  comma delimiter and returns `CsvRow[]` (i.e., `Record<string, string>[]`).
- `parseCSVWithDelimiter(source: string, delimiter: string)` — same with a configurable
  single-character delimiter (e.g., `"\t"` for TSV, `";"` for European CSV).
- Four-state machine: `FIELD_START`, `IN_UNQUOTED_FIELD`, `IN_QUOTED_FIELD`,
  `IN_QUOTED_MAYBE_END`.
- Full support for RFC 4180 quoted fields: commas, newlines, and `""` escape sequences
  inside quoted fields are handled correctly.
- Ragged row handling: short rows padded with `""`, long rows truncated to header length.
- Support for all three newline styles: `\n` (Unix), `\r\n` (Windows), `\r` (old Mac).
- `UnclosedQuoteError` class (extends `Error`) thrown when EOF is reached inside a
  quoted field. Includes `instanceof` fix via `Object.setPrototypeOf`.
- `CsvRow` type alias (`Record<string, string>`) and `ParseState` union type exported
  from `src/types.ts`.
- 56-test vitest test suite covering: basic tables, quoted fields with commas/newlines,
  escaped quotes, empty fields, ragged rows, empty input, header-only input, all three
  line ending styles, custom delimiters, whitespace preservation, lenient mode for
  malformed quoted fields, error handling.
- 100% statement, branch, function, and line coverage on all executable source files.
- No external runtime dependencies — pure TypeScript standard library only.
- Literate programming style: every function, state, and edge case documented inline
  with JSDoc, truth tables, and state machine diagrams.
