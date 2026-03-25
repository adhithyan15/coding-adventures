# Changelog — sql-csv-source (Go)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-03-25

### Added

- Initial release of `sqlcsvsource` package.
- `CSVDataSource` struct with `Schema` and `Scan` methods implementing the
  `sqlengine.DataSource` interface.
- `New(dir string) *CSVDataSource` constructor.
- Type coercion: empty string → nil, `"true"`/`"false"` → bool,
  parseable integer → int64, parseable float → float64, else string.
- `ParseHeader(tableName string) ([]string, error)` for reading column order
  directly from the raw CSV header line.
- Comprehensive end-to-end test suite with `testdata/` fixtures: full table
  scan, WHERE with typed values, IS NULL, INNER JOIN, GROUP BY/COUNT,
  ORDER BY/LIMIT, and `TableNotFoundError` on unknown table.
