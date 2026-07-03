# coding-adventures-xlsx-eval

**OOXML milestone M5** — evaluate the formulas in an `.xlsx` file.

This crate is a thin **bridge**: it takes a workbook parsed by
[`coding-adventures-spreadsheetml`](../spreadsheetml) (milestone M3) and
recomputes every formula from scratch using the existing
[`spreadsheet-core`](../spreadsheet-core) formula engine.

```text
bytes → … → spreadsheetml (M3)  → typed grid (cells + formula TEXT)
                                       │
                                       ▼   xlsx-eval (M5, THIS CRATE)
                             spreadsheet_core::Workbook (formulas RECOMPUTED)
```

## Why it exists

M3 reads a formula cell as `{ reference: "B2", value: Number(1000.0),
formula: Some("SUM(B1:B1)") }` — the `value` is the **cached** result the
authoring app last wrote. M3 never runs the formula.

M5 does the opposite: it **ignores the cached `<v>`**, feeds the formula *text*
to the engine, and lets the engine's parser + dependency graph + recalc produce
the value — exactly how a computing spreadsheet host behaves on open.

## Design: an opt-in adapter

* `spreadsheetml` stays dependency-light (just the OOXML reader stack). It has
  no formula engine and never should.
* `spreadsheet-core` is a complete formula engine that knows nothing about
  `.xlsx`.
* `xlsx-eval` depends on **both**, modifies **neither**, and is entirely
  opt-in. It owns *no* arithmetic — it only reshapes data (M3 `Value` → core
  `CellValue`, A1 string → `CellAddress`, formula text → `set_formula`). All
  formula semantics live in `spreadsheet-core`.

## Usage

```rust
use coding_adventures_xlsx_eval::{open_and_evaluate, computed_value};
use spreadsheet_core::CellValue;

# fn demo(bytes: &[u8]) {
let wb = open_and_evaluate(bytes).unwrap();

// B2 = SUM(B1:B1); the ENGINE computed this, not the stale cached value.
let v = computed_value(&wb, "Revenue", "B2");
assert_eq!(v, Some(CellValue::Number(1000.0)));
# }
```

### API

| Function | Purpose |
| --- | --- |
| `evaluate_workbook(&sml::Workbook) -> Result<core::Workbook, EvalError>` | Recompute an already-parsed M3 workbook. |
| `evaluate_workbook_verbose(&sml::Workbook) -> Result<Evaluation, EvalError>` | Same, plus non-fatal formula diagnostics. |
| `open_and_evaluate(&[u8]) -> Result<core::Workbook, EvalError>` | Open bytes (M3) then evaluate (M5). |
| `computed_value(&core::Workbook, sheet, a1) -> Option<CellValue>` | Ergonomic read by (sheet name, A1). |
| `sml_value_to_core(&sml::Value) -> CellValue` | Value conversion. |
| `parse_error_text(&str) -> SpreadsheetError` | `"#DIV/0!"` → sentinel. |

## Graceful degradation

A single formula the engine's parser cannot handle does **not** sink the
workbook: the cell falls back to its cached value and the failure is recorded as
a non-fatal `FormulaDiagnostic` (see `evaluate_workbook_verbose`). This mirrors a
real host, which shows the last cached value for a formula it can't parse.

## Out of scope

* Styles / number formats (milestone M4).
* Writing `.xlsx` back out.
* Named ranges, tables, 3-D refs — whatever `spreadsheet-core`'s parser does not
  yet accept falls back to its cached value with a diagnostic.

See `code/specs/SML03-formula-eval.md` for the full specification.
