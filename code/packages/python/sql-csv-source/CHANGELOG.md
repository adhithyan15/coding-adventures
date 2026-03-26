# Changelog — coding-adventures-sql-csv-source

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-03-25

### Added

- `CsvDataSource` — implements the `DataSource` ABC from `sql-execution-engine`,
  backed by CSV files in a directory. Each `tablename.csv` is one queryable table.
- `execute_csv(sql, directory)` — convenience wrapper that constructs a
  `CsvDataSource` and calls the execution engine in one step.
- Type coercion: empty string → `None`, `"true"`/`"false"` → `bool`,
  integer strings → `int`, float strings → `float`, else `str`.
- `schema()` reads column names from the CSV header row in order (first line
  split on comma, bypassing the dict to preserve insertion order).
- `scan()` uses `csv_parser.parse_csv` for full RFC 4180 support (quoted
  fields, embedded commas, escaped quotes).
- Full test suite with real CSV fixture files covering all 7 end-to-end
  query scenarios.
