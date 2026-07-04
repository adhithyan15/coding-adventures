# Changelog

All notable changes to `coding-adventures-sql-spreadsheet-source` are documented here.

## [0.1.0] — SSQL01: query a spreadsheet with SQL

### Added
- New crate: a `DataSource` (for `sql-execution-engine`) that exposes each sheet
  of a `spreadsheet-core::Workbook` as a SQL table — the sheet's first populated
  row is the column header, the rows below are the data.
- `SpreadsheetSource<'a>` (borrows a `&Workbook`) + `query(wb, sql) -> QueryResult`.
- `CellValue → SqlValue` mapping: integral numbers → `Int`, else `Float`; text →
  `Text`; bool → `Bool`; empty/error → `NULL`. Duplicate headers disambiguated
  with a `_2` suffix.
- Sparse iteration (`populated_cells`, never the dense used-range box), so a
  sheet spanning a huge range but holding few cells scans in O(populated).
- 13 tests + doctest: SELECT/WHERE(numeric+text)/GROUP BY+SUM/ORDER BY+LIMIT,
  multi-sheet-as-multi-table, unknown-table error, blank→NULL, duplicate headers,
  far-corner DoS guard, and an **end-to-end query over a `.xlsx` reopened via
  spreadsheet-io**. An example (`query_xlsx`) runs SQL over a file on disk.
