# Changelog

## 0.4.0

**Cell display formats.** `set_format(a1, code)` / `get_format(a1)` /
`get_display(a1)` expose the engine's cell-format API: set an Excel-style code
(empty clears it), read it back, and get the value rendered through it (e.g.
`1234.5` with `"#,##0.00"` → `"1,234.50"`). `get_display` is the display string a
cell paints — distinct from `get_value` (typed JSON) and `get_raw` (source). 1
new test.

## 0.3.0

**Insert/delete rows & columns.** `insert_rows` / `delete_rows` / `insert_cols`
/ `delete_cols(at, count)` (1-based) call through to the engine — which relocates
cells and rewrites every formula's references (a reference to a deleted line →
`#REF!`) — and keep this facade's `raw` echo map in step: each raw entry's
address is relocated the same way, and a formula's *source* is rewritten via the
shared `parse → FormulaAst::adjust → to_formula_string`, so the formula bar
echoes the post-edit references. An insert that would push a non-empty cell off
the u32 grid edge is rejected wholesale (mirrors the engine's guard). 3 new tests.

## 0.2.0

Viewport primitive (for the virtualized infinite sheet), wrapping
`spreadsheet-core` 0.2.0's reads as string-in/JSON-out:

- `get_window(row0, col0, row1, col1)` → `{"row0":..,"col0":..,"rows":R,"cols":C,
  "values":[[<value>,..],..]}` (row-major, blanks included as `{"kind":"empty"}`),
  or `{"error":"#REF!"}` on a bad/oversized request.
- `used_range()` → `{"minRow":..,"minCol":..,"maxRow":..,"maxCol":..}` or `null`.
- `column_letters(index)` → `"A"`/`"AA"`/… for a 1-based column index.
- `current_revision()` + `changed_since(since)` →
  `{"revision":N,"changed":["B2",..]}` or `{"revision":N,"stale":true}`.

## 0.1.0

Initial release — the browser/WASM-facing JSON facade over `spreadsheet-core`.

- `SpreadsheetSession`: a single-sheet session addressed by bare A1 strings.
- `set_cell(a1, raw)`: spreadsheet-style interpretation of a raw string
  (formula / boolean / number / text / clear), recomputing dependents.
- `get_value(a1)`, `get_values()`, `get_raw(a1)`: JSON value objects (shape
  matches the TypeScript engine's `CellValue` union) plus the typed source for
  the formula bar.
- Panic-safe (`catch_unwind`), JSON-escaped text values, non-finite numbers
  reported as `#NUM!`, oversized ranges surfaced as `#REF!`.
- 12 unit tests + a doctest; no WASM toolchain required to build or test.

Follows the repo's `macsyma-wasm` facade pattern. The `extern "C"` +
linear-memory ABI and the JS loader for the compiled `.wasm` land in a
follow-up.
