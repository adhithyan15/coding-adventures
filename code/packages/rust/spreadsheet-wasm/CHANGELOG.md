# Changelog

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
