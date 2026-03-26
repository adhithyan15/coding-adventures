# Changelog — sql_csv_source (Elixir)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-03-25

### Added

- Initial release of `CodingAdventures.SqlCsvSource`.
- `CsvDataSource` struct implementing the `CodingAdventures.SqlExecutionEngine.DataSource`
  behaviour backed by a directory of CSV files.
- Automatic type coercion: empty string → `nil`, `"true"`/`"false"` → boolean,
  parseable integer → `integer()`, parseable float → `float()`, else `String.t()`.
- `CodingAdventures.SqlCsvSource.new/1` convenience constructor and
  `CodingAdventures.SqlCsvSource.execute/2` one-liner for running SQL against a
  CSV directory.
- Comprehensive end-to-end test suite covering: full table scan, WHERE with typed
  values, IS NULL, INNER JOIN, GROUP BY / COUNT, ORDER BY / LIMIT, and error on
  unknown table.
