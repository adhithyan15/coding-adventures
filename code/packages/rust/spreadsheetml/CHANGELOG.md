# Changelog

All notable changes to `coding-adventures-spreadsheetml` are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-07-02

Initial release — OOXML milestone **M3** (SpreadsheetML workbook reader).

### Added

- `open_workbook(bytes: &[u8]) -> Result<Workbook, XlsxError>` — read an
  `.xlsx` from raw bytes into a typed cell grid, built on the `opc` (M2) and
  `xml-parser` (M1) crates.
- `Workbook` with `sheet_names()`, `sheet_by_name(&str)`, and `sheets()`.
- `Sheet` with `cell(a1: &str)`, `cells()` (populated cells in row-major reading
  order), and `cell_count()`.
- `Cell { reference, value, formula }` — the A1 ref, decoded value, and optional
  formula text (with the cached value surfaced as the cell's value; formulas are
  **not** evaluated at this milestone).
- `Value` enum: `Number(f64)`, `Text(String)`, `Bool(bool)`, `Error(String)`,
  `Empty`. Shared strings and inline strings both surface as `Text`.
- `parse_a1_ref(a1: &str) -> Option<(u32, u32)>` — parse an A1 reference into
  `(col, row)`, both 1-based, using bijective base-26 for the column letters.
- `XlsxError` enum wrapping `OpcError` and covering a missing workbook, non-UTF-8
  parts, malformed XML, an unresolvable sheet `r:id`, and an out-of-range
  shared-string index.

### Semantics implemented

- Resolves the `r:id` → part indirection: `<sheet r:id="rId1">` is dereferenced
  through OPC relationships to the worksheet part.
- Resolves the shared-string indirection: `t="s"` cells dereference their `<v>`
  index into the shared-string table. A missing `sharedStrings` part is legal
  (empty table). Rich-text `<si>` entries concatenate all descendant `<t>` runs.
- Decodes every `t` variant: number (`n`/absent), shared string (`s`), formula
  string result (`str`), inline string (`inlineStr`), boolean (`b`), error
  (`e`), and blank cells.

### Not yet (deferred to M4)

- Styles, number formats, and date/time interpretation — numbers are returned
  raw.
- Formula evaluation — formula text + cached value only.
- Writing `.xlsx`.

### Tests

- 27 unit/integration tests + 1 doctest, all passing, including an end-to-end
  test over a real DEFLATE-compressed `.xlsx` fixture asserting the full
  Revenue-sheet grid.
