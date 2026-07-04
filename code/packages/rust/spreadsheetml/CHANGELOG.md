# Changelog

All notable changes to `coding-adventures-spreadsheetml` are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] — 2026-07-03

### Fixed
- **Read real-world files with uncomputed formulas.** A numeric/formula cell
  whose cached value element is present but empty (`<c><f>SUM(..)</f><v></v></c>`)
  no longer errors with `bad number ""`. openpyxl (and other producers that
  don't evaluate) emit exactly this; the `<v>` is only a cache, so an empty one
  decodes as `Value::Empty` and the formula text is still captured for
  re-evaluation. Surfaced while wiring `spreadsheet-io` (SSIO01), whose committed
  openpyxl fixture guards it.

## [0.2.0] — 2026-07-02

OOXML milestone **M4** — number formats, dates, merged cells, defined names.
Fully **backward-compatible** with M3: `Value` and every existing signature are
unchanged, so a date cell still holds `Value::Number(45292.0)`; the format is
attached *alongside* the raw value, never applied to it.

### Added

- `xl/styles.xml` interpretation. The styles part is resolved via the workbook's
  `.../relationships/styles` relationship and parsed into a `StyleTable`
  (`<numFmts>` custom-format map + ordered `<cellXfs>`).
- The built-in number-format id table (ECMA-376 §18.8.30): ids `< 164` map to
  their spec-defined format codes (`14 → m/d/yyyy`, `9 → 0%`, `49 → @`, …) via
  `builtin_format_code(id)`.
- `NumberFormatKind` enum (`General`, `Number`, `Date`, `Time`, `DateTime`,
  `Percent`, `Currency`, `Text`, `Other`) plus classifiers `classify_id(id)` and
  `classify_format_code(code)` (custom codes are inferred from their tokens,
  respecting quoted/bracketed/escaped literal contexts).
- `NumberFormat { id, code, kind }` and `Cell::number_format: Option<NumberFormat>`
  resolved from the cell's `s=` style index. `None` for unstyled, `General`, or
  out-of-range style indices (graceful).
- Date interpretation in the **1900 date system** (serial 0 = 1899-12-30,
  reproducing Excel's phantom 1900-02-29 leap-year bug for serials ≥ 60):
  `serial_to_date(f64) -> Option<String>` (ISO `YYYY-MM-DD`) and
  `serial_to_datetime(f64) -> Option<String>` (ISO `YYYY-MM-DDTHH:MM:SS`).
- `Cell` methods: `format_kind()`, `as_date()`, `as_datetime()`, and a pragmatic
  `formatted()` (exact ISO for dates; `×100 + "%"` for percent; raw number for
  currency — a full number-format renderer is out of scope).
- Merged cells: `Sheet::merged_ranges() -> &[CellRange]` from `<mergeCells>`, and
  a `CellRange { start, end }` type with `CellRange::parse("A1:B1")`.
- Defined names: `Workbook::defined_names() -> &[(String, String)]` from
  `<definedNames>` (name → raw reference text; not evaluated).

### Semantics implemented

- The `s=` → `cellXfs[s]` → `numFmtId` → format-code → kind chain, with custom
  (`≥ 164`) codes read from `<numFmts>` and built-ins hard-coded.
- The famous "the number 45292 is actually 2024-01-01" is now recoverable via
  `Cell::as_date()`.

### Tests

- 45 unit/integration tests + 3 doctests, all passing. Adds a real
  DEFLATE-compressed styled `.xlsx` fixture (`STYLED_XLSX`) asserting the
  date/currency/percent/merged/defined-name end-to-end, plus unit coverage of
  the built-in id table, custom-code classification, and serial→date edge cases
  (serial 1, 60, 61, 45292). All original M3 tests pass unchanged.

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
