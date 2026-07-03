# SML03 — Evaluate `.xlsx` formulas (`xlsx-eval`)

## Overview

This is milestone **M5** of the OOXML effort. It builds a Rust crate,
`coding-adventures-xlsx-eval`, that takes a workbook parsed by
[`coding-adventures-spreadsheetml`](SML01-spreadsheetml.md) (milestone M3) and
**recomputes** every formula cell from scratch using the existing
[`spreadsheet-core`](../packages/rust/spreadsheet-core) engine.

Where M3 stops:

```text
raw bytes (.xlsx)
      |
      v
spreadsheetml (M3)  → Workbook / Sheet / Cell / Value   (cells + formula TEXT)
      |                     - a formula cell carries its <f> text plus its
      |                       *cached* <v> value; M3 NEVER evaluates
      v
xlsx-eval (M5, HERE) → spreadsheet_core::Workbook  (formulas RECOMPUTED)
```

M3 reads a formula cell as `{ reference: "B2", value: Number(1000.0),
formula: Some("SUM(B1:B1)") }` — the `value` is the *cached* result the
authoring application last wrote to disk. It does **not** run the formula.

M5 is the opposite: it **ignores the cached `<v>`** for formula cells, feeds the
formula *text* to the `spreadsheet-core` engine, and lets the engine's own
parser + dependency graph + recalc produce the value. This is how a real
spreadsheet host behaves on open: the cached values are a courtesy for viewers
that can't compute, but a computing host recalculates.

## Why a separate crate (the adapter pattern)

`spreadsheetml` is deliberately dependency-light: it depends only on the OOXML
reader stack (`opc`, `xml-parser`). It has **no** formula engine and never
should — reading bytes and evaluating arithmetic are different concerns.

`spreadsheet-core` is a complete, self-contained formula engine: cell model,
Pratt formula parser, dependency DAG, recalc, and a dispatch table into the
Layer-1 math cores (`statistics-core` supplies `SUM`, etc.). It knows nothing
about `.xlsx`, ZIP, or XML.

`xlsx-eval` is the **thin bridge** between the two. It:

* depends on *both* crates (path deps),
* modifies *neither*,
* is an **opt-in** layer — a caller who only wants the raw grid uses M3 alone
  and never pays for the engine.

Keeping evaluation out of `spreadsheetml` also keeps M5 independent of the
in-flight M4 (styles / number formats), which touches `spreadsheetml`.

## What the adapter does

`evaluate_workbook(sml: &spreadsheetml::Workbook) -> Result<spreadsheet_core::Workbook, EvalError>`:

1. Create an empty `spreadsheet_core::Workbook`.
2. **First pass — create all sheets in order.** Each M3 sheet name becomes a
   `spreadsheet_core` sheet via `add_sheet(name) -> SheetId`. All sheets are
   created *before* any cell is filled so that a cross-sheet formula
   (`Sheet2!A1`) can resolve its target sheet — the engine resolves sheet names
   through its own internal `name → SheetId` map, which must be fully populated
   first.
3. **Second pass — fill every populated cell.** For each cell, parse its A1
   `reference` to a `spreadsheet_core::CellAddress`. Then:
   * If the cell has `formula: Some(text)` → `set_formula(sheet, addr, &text)`.
     The engine's parser accepts the bare `<f>` body (leading `=` is optional).
     On a *parse error*, we do **not** abort the whole workbook — one bad
     formula must not kill the sheet. Instead we fall back to setting the cell's
     **cached value** as a literal (`set_value`) and record the failure in a
     per-cell diagnostics list on the result. This mirrors a real host, which
     shows the last cached value for a formula it cannot parse.
   * Else → `set_value(sheet, addr, convert(cell.value))`.
4. `recalc_all()` — the engine topologically evaluates every formula (cycles
   collapse to `#REF!`; oversized ranges are already capped at 2²⁰ cells).
5. Return the hydrated `spreadsheet_core::Workbook`.

### Value conversion

`sml_value_to_core(&spreadsheetml::Value) -> spreadsheet_core::CellValue`:

| M3 `Value`      | core `CellValue`                    |
| --------------- | ----------------------------------- |
| `Number(f)`     | `Number(f)`                         |
| `Text(s)`       | `Text(s)`                           |
| `Bool(b)`       | `Boolean(b)`                        |
| `Empty`         | `Empty`                             |
| `Error(s)`      | `Error(parse_error_text(s))`        |

`parse_error_text(&str) -> SpreadsheetError` maps the disk error string to the
engine's sentinel:

| string      | sentinel        |
| ----------- | --------------- |
| `#DIV/0!`   | `DivZero`       |
| `#N/A`      | `NotAvailable`  |
| `#NAME?`    | `Name`          |
| `#NUM!`     | `Num`           |
| `#REF!`     | `Ref`           |
| `#VALUE!`   | `Value`         |
| `#NULL!`    | `Null`          |
| anything else | `Value` (a safe default) |

## Public surface

* `pub fn evaluate_workbook(sml: &spreadsheetml::Workbook) -> Result<spreadsheet_core::Workbook, EvalError>`
* `pub fn open_and_evaluate(bytes: &[u8]) -> Result<spreadsheet_core::Workbook, EvalError>`
  — `open_workbook` then `evaluate_workbook`.
* `pub fn computed_value(wb: &spreadsheet_core::Workbook, sheet: &str, a1: &str) -> Option<CellValue>`
  — an ergonomic reader by (sheet name, A1) for tests / CLIs.
* `EvalError` — wraps `XlsxError` (open failure) and an invalid-A1 adapter error.

Note that formula-parse failures are **not** `EvalError`s: they are recorded,
non-fatal diagnostics, because the workbook is still usable.

## Ownership of formula semantics

`xlsx-eval` owns **none** of the arithmetic. It is purely a data-shape adapter:
M3 `Value` → core `CellValue`, M3 A1 string → core `CellAddress`, M3 formula
text → the engine's `set_formula`. Every question of "what does `SUM(B1:B1)`
mean, what is its precedence, how does a range expand, what happens on a cycle"
is answered entirely inside `spreadsheet-core`. This keeps the semantics in one
place and lets the adapter stay tiny and obviously correct.

## Out of scope

* Styles / number formats (M4) — a cell that *displays* as currency still
  evaluates as its bare number.
* Writing `.xlsx` back out (a future milestone).
* Named ranges, tables, 3-D refs — whatever `spreadsheet-core`'s parser does
  not yet accept, a formula using it falls back to its cached value with a
  recorded diagnostic.
