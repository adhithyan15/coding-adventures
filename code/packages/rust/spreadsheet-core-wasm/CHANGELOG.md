# Changelog

## 0.7.0

**Clipboard — cut / copy / paste.** `copy(start, end)` / `cut(start, end)` capture the inclusive rectangle into the engine's clipboard and mirror each cell's raw source into a facade-side `RawClip`; `paste(dst_start) -> bool` places the block at `dst_start`, shifting the whole block's references by the destination's offset and keeping the `raw` echo in step (each target's source is the shifted source; blanks erase). A cut clears the source echo it didn't overwrite and consumes the buffer; a copy is repeatable. `has_clipboard()` reports whether a block is held. The facade mirrors the engine's `MAX_RANGE_CELLS` guard and i64-clamped delta, and tracks the engine's buffer lifecycle exactly (kept on reject/copy, dropped on cut-paste). 3 tests.

## 0.6.0

**`fill` (drag-fill) over the JSON facade.** New
`fill(src_a1, dst_start_a1, dst_end_a1)` replicates the source cell across the
inclusive A1 rectangle, wrapping the engine's `Workbook::fill` (spreadsheet-core
0.7.0): relative references shift per target, absolute (`$`) refs pin, the
source's format carries along, an empty source clears each target, and a
malformed address is a no-op.

- Keeps the `raw` echo map honest: each target's stored source is the source's
  source with its references shifted (`parse → shift → serialize`, the
  copy/paste sibling of `rewrite_raw_for_edit`), so the formula bar shows the
  filled formula. Offsets computed in i64 then clamped (matching the engine, so
  a high-coordinate anchor can't overflow), and the facade mirrors the engine's
  `MAX_RANGE_CELLS` guard so the raw-map loop also stays bounded.
- 3 new tests (formula shift + echo, literal copy / clear-from-empty, bad-address
  no-op).

## 0.5.0

**`get_display_window`** — a windowed read returning each cell's **formatted
display string**, the format-aware sibling of `get_window`. Wraps the engine's
`Workbook::get_display_window` (added in `spreadsheet-core` 0.6.0) at the JSON
boundary so a virtualized host fetches a dense, ready-to-paint rectangle per
frame instead of re-deriving number formatting in JS.

- `get_display_window(row0, col0, row1, col1) -> String` →
  `{"row0":1,"col0":1,"rows":R,"cols":C,"cells":[["1,234.50",…],…]}`, a row-major
  `R×C` array of display strings (empty cells `""`). A bad request
  (inverted / oversized / 0-coord) returns `{"error":"#REF!"}`, mirroring
  `get_window`.
- 1 new test (formatted/percent/text/empty cells row-major + the bounds guards).

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
