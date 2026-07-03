# Changelog

## [0.1.0] — 2026-07-03

### Added

- Initial release of the `read-xlsx` CLI — the runnable end-goal of the OOXML
  effort. Opens a real `.xlsx` and prints each sheet as a grid of **evaluated**
  cell values, exercising the full home-grown, zero-third-party-dependency stack
  (`zip` → `deflate` → `xml-parser` → `opc` → `spreadsheetml` → `xlsx-eval`).
- `read-xlsx <file.xlsx>` reads a file; `read-xlsx --demo` runs two embedded real
  `.xlsx` fixtures (a formula workbook and a styled workbook); `read-xlsx --help`.
- Library API: `render_xlsx(bytes) -> Result<Vec<RenderedSheet>, RenderError>` and
  `format_report(&sheets) -> String`, so the rendering is reusable and testable.
- Output shows recomputed formula values (cached `<v>` ignored) and number-format
  display (serial `45292` → `2024-01-01`, `0.25` → `25%`, currency).
- 4 tests covering the formula recomputation, date/percent formatting, the
  text-report formatter, and a graceful error on non-`.xlsx` bytes.
