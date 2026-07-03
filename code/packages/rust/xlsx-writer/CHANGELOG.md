# Changelog

All notable changes to `coding-adventures-xlsx-writer` are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/); this
project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-07-03

Initial release — the write side of OOXML milestone **C1**, mirroring the
read-side `spreadsheetml` crate.

### Added

- `Workbook` / `Sheet` model:
  - `Workbook::new`, `add_sheet(name)` (sheets keep insertion order).
  - `Sheet::set_number`, `set_string`, `set_formula` (formula text without `=`,
    plus a cached value). A malformed A1 reference is a silent no-op.
- `write_xlsx(&Workbook) -> Vec<u8>` — generates the SpreadsheetML parts
  (`workbook.xml`, `worksheets/sheetN.xml`, `sharedStrings.xml`, the two `.rels`
  parts) and packages them via `opc-writer`.
  - Text cells are deduplicated into a shared-string table with correct
    `count` / `uniqueCount`.
  - `sharedStrings.xml` is omitted entirely when there are no text cells.
  - Numbers render without a trailing `.0`; non-finite values emit `0`.
  - Sheet names, cell text, and formula text are all XML-escaped.
- Public A1 helpers: `parse_a1`, `col_to_letters`.
- **Round-trip proof** (`tests/round_trip.rs`): writes a "Revenue" workbook and
  re-opens the bytes with the repo's own `spreadsheetml` (structural) and
  `xlsx-eval` (formula recompute) readers. Also covers multiple sheets with
  shared-string dedup, XML-special characters, Unicode, and a cross-sheet
  formula that recomputes.

### Notes

- `#![forbid(unsafe_code)]`; no `unwrap`/`expect`/`panic!` on input paths.
