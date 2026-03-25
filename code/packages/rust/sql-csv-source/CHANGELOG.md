# Changelog — coding-adventures-sql-csv-source

All notable changes are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-03-25

### Added

- `CsvDataSource` — implements the `DataSource` trait from
  `coding-adventures-sql-execution-engine`, backed by CSV files in a directory.
- `schema()` reads the first line of the CSV directly (bypassing HashMap to
  preserve column order from the header row).
- `scan()` uses `parse_csv` from `coding-adventures-csv-parser` for full
  RFC 4180 support, then coerces each value via `coerce()`.
- Type coercion: `""` → `None`, `"true"`/`"false"` → `SqlPrimitive::Bool`,
  parseable i64 → `SqlPrimitive::Int`, parseable f64 → `SqlPrimitive::Float`,
  else `SqlPrimitive::Text`.
- Integration test suite using real CSV fixture files covering all 7 query
  scenarios.
