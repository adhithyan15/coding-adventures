# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] - 2026-03-31

### Changed

- Wrapped public functions `New` and `Parse` with the Operations system (`StartNew[T]`), providing automatic timing, structured logging, and panic recovery. `Parse` delegates to a private `parseInternal` helper to keep the wrapping clean. Public API signatures unchanged.

## [0.1.0] - 2026-03-23

### Added

- Initial implementation of `Parser` with a `Parse([]byte) (*WasmModule, error)` method
  that decodes a raw `.wasm` binary into a structured `*WasmModule`.
- `ParseError` struct with `Message` and `Offset` fields for precise error
  reporting; implements the `error` interface.
- Full parsing support for all 12 WASM 1.0 section types:
  - Section 0: Custom (name + arbitrary data)
  - Section 1: Type (function type signatures)
  - Section 2: Import (function, table, memory, global imports)
  - Section 3: Function (type indices for local functions)
  - Section 4: Table (funcref table definitions with limits)
  - Section 5: Memory (linear memory definitions with limits)
  - Section 6: Global (global variable declarations with init expressions)
  - Section 7: Export (exported functions, tables, memories, globals)
  - Section 8: Start (auto-called function on instantiation)
  - Section 9: Element (table initialization segments)
  - Section 10: Code (function bodies with local declarations and bytecode)
  - Section 11: Data (memory initialization segments)
- Header validation: magic bytes (`\0asm`) and version (1) are checked.
- Section ordering enforcement: sections 1–11 must appear in ascending ID order.
- LEB128 bounds checking throughout; all reads guard against truncated input.
- 46 unit tests covering all section types, import kinds, error paths, and a
  full round-trip test. Test coverage: **81.6%** (exceeds 80% threshold).
- Literate programming style: Go doc comments with ASCII art, design rationale,
  and worked examples throughout the source code.
- `go vet` passes with no warnings.
