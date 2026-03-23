# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- Initial implementation of `CodingAdventures.WasmModuleParser` — parses raw WASM binary
  bytes into a structured `WasmModule` using idiomatic Elixir binary pattern matching.
- Full header validation: magic `\0asm` (<<0x00, 0x61, 0x73, 0x6D>>) and version 1
  (<<0x01, 0x00, 0x00, 0x00>>).
- Section parsers for all 12 WASM 1.0 section types:
  - §0 Custom: name + raw data binary
  - §1 Type: function signatures (FuncType)
  - §2 Import: function/table/memory/global imports
  - §3 Function: type index list
  - §4 Table: funcref tables with limits
  - §5 Memory: linear memory with limits
  - §6 Global: globals with init_expr binary
  - §7 Export: named exports (function/table/memory/global)
  - §8 Start: optional function index
  - §9 Element: table initialisation segments
  - §10 Code: function bodies with expanded local declarations
  - §11 Data: memory initialisation segments
- `read_expr` helper that reads constant expressions byte-by-byte until `end` opcode
  (0x0B), with correct immediate handling for i32.const, i64.const, f32.const, f64.const,
  and global.get.
- Returns `{:ok, %WasmModule{}}` or `{:error, String.t()}`.
- 41 unit tests covering all sections, all error cases, and a round-trip test.
- 87.92% test coverage (exceeds the 80% threshold).
- Literate programming style: ASCII format diagrams and detailed inline documentation.
