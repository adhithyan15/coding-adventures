# Changelog

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
