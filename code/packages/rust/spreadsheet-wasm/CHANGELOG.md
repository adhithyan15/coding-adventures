# Changelog

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
