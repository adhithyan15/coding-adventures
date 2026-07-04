# Changelog

All notable changes to `spreadsheet-io` are documented here.

## [0.1.0] — SSIO01: unify file I/O onto spreadsheet-core (.xlsx)

### Added
- New crate `spreadsheet-io`, the adapter layer that makes
  `spreadsheet-core::Workbook` the single in-memory model every spreadsheet file
  format converts through.
- `load_xlsx(&[u8]) -> Result<Workbook, IoError>` — open a `.xlsx` into a live
  engine workbook via `spreadsheetml` + `xlsx-eval`. Formulas are installed as
  formulas (not flattened to values) and recomputed.
- `save_xlsx(&Workbook) -> Vec<u8>` — serialize an engine workbook to `.xlsx`
  bytes via `xlsx-writer`, driven off `populated_cells` / `cell_is_formula` /
  `cell_source_text` / `get_value`. Formula cells with numeric results are
  written as `<f>` + cached `<v>`; leading `=` is stripped for the `<f>` body.
  Walks each sheet's populated cells **sparsely** (never the dense bounding box),
  so a workbook spanning a huge range but holding few cells saves in O(cells) —
  closing a DoS a security review flagged (a two-cell `A1`/`XFD1048576` sheet
  would otherwise force ~17 billion iterations).
- `IoError` (with `Display` + `std::error::Error`) for load failures.
- 13 tests: value + formula + sheet-order round-trips, live recompute after
  reopen, save idempotence, XML-metacharacter text, empty workbook, the
  documented boolean-as-number behaviour, and non-`.xlsx` rejection.

### Changed (dependency crate)
- `spreadsheet-core`: added `Workbook::cell_is_formula(sheet, addr) -> bool`, the
  read accessor a serializer needs to tell a formula from a literal (the `=`
  prefix is unreliable and `cell_source_text` alone can't distinguish them).

### Known limitations
- The `.xlsx` writer's value model can't express boolean cells (written as
  `1`/`0`), non-numeric formula results (written as the computed value, losing
  the formula), or number formats (not yet emitted). Numbers, text, and
  numeric-result formulas round-trip exactly.

### Not yet
- `.xls` load/save (SSIO02); `SpreadsheetSession` + WASM wiring (SSIO03–04);
  VisiCalc open/save UI (SSIO05).
