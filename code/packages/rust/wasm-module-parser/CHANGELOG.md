# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- Initial implementation of `WasmModuleParser` — parses raw `.wasm` binary bytes into a
  structured `WasmModule` with no execution.
- `WasmParseError` type with `message` and `offset` fields; implements `Display` and
  `std::error::Error`.
- Full header validation: magic `\0asm` (0x00 0x61 0x73 0x6D) and version 1
  (0x01 0x00 0x00 0x00).
- Section parsers for all 12 WASM 1.0 section types:
  - §0 Custom: name + raw data
  - §1 Type: function signatures (FuncType)
  - §2 Import: function/table/memory/global imports
  - §3 Function: type index array
  - §4 Table: funcref tables with limits
  - §5 Memory: linear memory with limits
  - §6 Global: globals with init_expr (constant expression)
  - §7 Export: named exports (function/table/memory/global)
  - §8 Start: optional function index
  - §9 Element: table initialisation segments
  - §10 Code: function bodies with expanded local declarations
  - §11 Data: memory initialisation segments
- Internal `Parser` struct with cursor-tracked position for precise error offsets.
- `read_expr` helper that reads constant expressions (init_expr / offset_expr) byte-by-byte
  until the `end` opcode (0x0B), with correct immediate parsing for i32.const, i64.const,
  f32.const, f64.const, and global.get.
- 28 unit tests covering all sections, all error cases, and a round-trip test.
- Literate programming style: ASCII format diagrams, per-section explanations, and
  Knuth-style inline documentation throughout.
