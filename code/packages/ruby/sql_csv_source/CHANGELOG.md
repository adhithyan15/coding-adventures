# Changelog — coding_adventures_sql_csv_source

All notable changes are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-03-25

### Added

- `CsvDataSource` — implements the DataSource mixin from
  `coding_adventures_sql_execution_engine`, backed by CSV files in a directory.
  Each `tablename.csv` is one queryable table.
- Type coercion: `""` → `nil`, `"true"`/`"false"` → boolean,
  integer strings → `Integer`, float strings → `Float`, else `String`.
- `schema()` reads the first line of the CSV file directly for ordered
  column names (avoids any hash-ordering ambiguity).
- `scan()` uses `CodingAdventures::CsvParser.parse_csv` for full RFC 4180
  support including quoted fields and embedded commas.
- Full test suite against real CSV fixtures covering all 7 query scenarios.
