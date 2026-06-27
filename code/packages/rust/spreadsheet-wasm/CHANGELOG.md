# Changelog

## 0.12.0

**Multi-sheet WASM exports.** Linear-memory exports for the sheet ops:
`sheet_names` (packed JSON), `active_sheet` (u32), `set_active_sheet`, `add_sheet`,
`rename_sheet`, `delete_sheet`, `move_sheet` (flag i32). The web demo's JS loader
gains `sheetNames`/`activeSheet`/`setActiveSheet`/`addSheet`/`renameSheet`/
`deleteSheet`/`moveSheet` wrappers; the committed `.wasm` + bundle are rebuilt.

## 0.11.0

**Find / replace exports.** `find_all(query_ptr, query_len, in_formulas, match_case)
-> *mut u8` returns a packed JSON string `{"matches":[…]}`; `replace_all(query…,
repl…, match_case) -> i32` returns the count of cells changed. The committed
`pkg/spreadsheet_engine.wasm` is rebuilt to carry both. → 0.11.0.

## 0.10.0

**Range sort — `sort_range` export.** New linear-memory export
`sort_range(start_ptr, start_len, end_ptr, end_len, key_col, ascending) -> i32` over
the facade's `sort_range`: reorder the rows of the rectangle by the computed values in
`key_col`; `ascending` is a flag (0 = descending). Returns 1 when applied (or already
sorted), 0 for a malformed address / out-of-range `key_col` / empty-single-row /
oversized range. The committed `pkg/spreadsheet_engine.wasm` is rebuilt to carry the
new export; the JS host re-reads via `get_window` afterwards.

## 0.9.0

**Undo / redo (session history).** New zero-arg linear-memory exports `undo()`, `redo()`, `can_undo()`, `can_redo()` — each returns `i32` (1/0), no string marshalling. JS loader gains `undo()` / `redo()` / `canUndo()` / `canRedo()` (each → `boolean`). Rebuilt `pkg/spreadsheet_engine.wasm`. `js/smoke.mjs` extended: make two edits, undo both (B1 then A1 gone), redo both (the formula recomputes live → 10), and confirm `canUndo`/`canRedo` gate correctly and a redo at the top is a no-op. Delegates to spreadsheet-core-wasm 0.9.0's snapshot-based history.

## 0.8.0

**Save / load (serialize).** New linear-memory exports `serialize() -> *u8` (a packed JSON document of the workbook's source + formats; read via the existing output convention) and `deserialize(data*, len) -> i32` (1 = loaded, 0 = malformed / unsupported version, leaving the workbook untouched). JS loader gains `serialize() -> string` and `deserialize(data) -> boolean`. Rebuilt `pkg/spreadsheet_engine.wasm`. `js/smoke.mjs` extended: serialize the workbook → load into a fresh session → confirm a moved cell and a live formula (edit F3 → G3 = F3*2 recomputes), and that garbage input is rejected.

## 0.7.0

**Clipboard — cut / copy / paste.** New linear-memory exports `copy(start*, len, end*, len)`, `cut(...)` (void), and `paste(dst*, len) -> i32` (1/0). JS loader gains `copy(start, end)`, `cut(start, end)`, and `paste(dstStart) -> boolean`. Rebuilt `pkg/spreadsheet_engine.wasm`. `js/smoke.mjs` extended: copy F1:G1 → paste at F3 (G3 = F3*2, echo `=(F3*2)`), cut A1 → move to C1 (source clears, second paste returns false).

## 0.6.0

**`fill` (drag-fill) over the WASM ABI.** New
`fill(src, src_len, dst_start, dst_start_len, dst_end, dst_end_len)` export
(three `(ptr, len)` A1 strings, void return), delegating to the thread-local
`SpreadsheetSession`. Replicates the `src` cell across the inclusive rectangle —
relative refs shift per target, absolute (`$`) refs pin, the source's format
carries along, an empty source clears each target; a malformed address is a
no-op. The JS loader gains a `fill(src, dstStart, dstEnd)` method, the committed
`pkg/spreadsheet_engine.wasm` is rebuilt, and `js/smoke.mjs` drives it against
the real module (filled value + echoed shifted source). 1 new ABI round-trip
test.

## 0.5.0

**`get_display_window` over the WASM ABI.** New
`get_display_window(row0, col0, row1, col1) -> *mut u8` export (integer coords
passed directly, packed JSON result), delegating to the thread-local
`SpreadsheetSession`. Like `get_window` but each cell is its display string
(value rendered through its format code; empty cells `""`) — the per-frame read
for a virtualized grid. The ABI round-trip test now also drives it (formatted
cell + bad-window guard).

Also rounds out the JS loader (`js/spreadsheet-engine-wasm.mjs`) to mirror the
full ABI: it now surfaces `getDisplayWindow` plus the format trio (`setFormat` /
`getFormat` / `getDisplay`, ABI exports since 0.4.0) and the structural edits
(`insertRows` / `deleteRows` / `insertCols` / `deleteCols`, since 0.3.0) — which
were compiled into the `.wasm` but not yet exposed in the loader. The committed
`pkg/spreadsheet_engine.wasm` is rebuilt, and `js/smoke.mjs` now drives
`getDisplayWindow` against the real module (formatted display strings +
bad-window guard).

## 0.4.0

**Cell display formats over the WASM ABI.** New `set_format(a1, code)` (void),
`get_format(a1)`, and `get_display(a1)` exports (pointer/length string
marshalling via `read_input`/`pack`), delegating to the thread-local
`SpreadsheetSession`. `get_display` returns the value rendered through its format
code. 1 new ABI round-trip test.

## 0.3.0

**Insert/delete rows & columns over the WASM ABI.** New `void`-returning exports
`insert_rows` / `delete_rows` / `insert_cols` / `delete_cols(at, count)` (1-based,
plain integer args — no pointer marshalling), each delegating to the thread-local
`SpreadsheetSession`. The JS host re-reads via `get_window` / `get_raw`
afterwards. 1 new ABI round-trip test (insert then delete restores the sheet).

## 0.2.0

Viewport entry points for the virtualized infinite sheet, mirroring
`spreadsheet-core-wasm` 0.2.0. Integer coordinates are passed directly (no
pointer marshalling); JSON results use the existing `[len][bytes]` pack:

- `get_window(row0, col0, row1, col1) -> *mut u8` (packed window JSON).
- `used_range() -> *mut u8`, `column_letters(index) -> *mut u8`.
- `current_revision() -> u64` (returned directly, not packed).
- `changed_since(since: u64) -> *mut u8`.

## 0.1.0

Initial release — the `extern "C"` + linear-memory WASM ABI over the
`spreadsheet-core-wasm` JSON facade.

- Hand-written `#[no_mangle] extern "C"` exports (zero-dependency; no
  `wasm-bindgen`): `alloc`, `dealloc`, `reset`, `set_cell`, `get_value`,
  `get_raw`, `get_values`.
- Linear-memory string protocol: inputs as `(ptr, len)`, outputs as
  `[len: u32 LE][utf8]` buffers. Allocation and free use a matched explicit
  `Layout(align = 1)`, so they can never disagree on size. Allocation failure
  returns null instead of panicking (which would trap the module).
- A single global `SpreadsheetSession` in thread-local storage; `reset` starts
  a fresh sheet.
- `build-wasm.sh` compiles to `pkg/spreadsheet_engine.wasm` (committed
  artifact). `js/spreadsheet-engine-wasm.mjs` is a dependency-free loader that
  presents the **same API as the TypeScript engine** (`createSpreadsheet` →
  `setCell`/`getValue`/`getRaw`/`getValues`) and runs in Node and the browser.
- 4 host-target tests drive the `(ptr, len)` protocol by hand; `js/smoke.mjs`
  loads the real `.wasm` and verifies it computes the same results as the Rust
  and TypeScript engines (SUM = 46, AVERAGE = 9.2, precedence, incremental
  recalc, error propagation, JSON-escaped text). Zero clippy warnings.

Next: the HTML and WebComponent VisiCalc demos consume this `.wasm`.
