# Changelog — @coding-adventures/sql-csv-source

All notable changes are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-03-25

### Added

- `CsvDataSource` — implements the `DataSource` interface from
  `@coding-adventures/sql-execution-engine`, backed by CSV files in a directory.
- `schema()` reads the first line of the CSV file directly (split on comma)
  to preserve column header order — Object.keys() order is preserved in V8
  but reading the header line directly is more explicit and safe.
- `scan()` uses `parseCSV` from `@coding-adventures/csv-parser` for full
  RFC 4180 support, then coerces each value with the `coerce()` helper.
- Type coercion: `""` → `null`, `"true"`/`"false"` → boolean,
  integer strings → `number`, float strings → `number`, else `string`.
- Uses `node:fs` `readFileSync` and `node:path` `join` for file I/O —
  standard Node.js built-ins with no additional runtime dependencies.
- Full test suite using real CSV fixture files, located via `import.meta.url`
  for ESM compatibility, covering all 7 end-to-end query scenarios.
