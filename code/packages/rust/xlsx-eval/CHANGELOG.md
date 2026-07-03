# Changelog

All notable changes to `coding-adventures-xlsx-eval` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-07-02

### Added

- Initial release — **OOXML milestone M5** (`code/specs/SML03-formula-eval.md`).
- `evaluate_workbook(&spreadsheetml::Workbook) -> Result<spreadsheet_core::Workbook, EvalError>`:
  recomputes every formula in an M3-parsed workbook from scratch using the
  `spreadsheet-core` engine, ignoring the cached `<v>` on disk.
- `evaluate_workbook_verbose` returning an `Evaluation { workbook, diagnostics }`
  so callers can see which formulas the engine could not parse.
- `open_and_evaluate(&[u8])` convenience: open `.xlsx` bytes (M3) then evaluate.
- `computed_value(&workbook, sheet, a1)` ergonomic reader by (sheet name, A1).
- `sml_value_to_core` value conversion and `parse_error_text` error-code mapping.
- Two-pass hydration: all sheets are created before any cell is filled so
  cross-sheet formula references resolve.
- Graceful degradation: a formula the engine cannot parse falls back to its
  cached value and is recorded as a non-fatal `FormulaDiagnostic` — one bad
  formula never sinks the workbook.
- End-to-end test proving `SUM(B1:B1)` in `MINIMAL_XLSX` recomputes to `1000`
  from scratch, plus a hand-built book whose stale cached values (`999`, `0`)
  are correctly overridden by the engine's recomputed `60` and `20`.
- Unit tests for value/error conversion (every variant), A1 argument order,
  garbage refs, empty workbooks, and open-error handling. Coverage well over 80%.
