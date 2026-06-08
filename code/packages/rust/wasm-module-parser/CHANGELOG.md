# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] — 2026-06-04

### Added — WasmGC struct types in the type section (LANG77 / McCarthy L3b-3a-3c)

The type section now parses **WasmGC struct types**, not just function types, so
a module that defines a `$LispyPair` cons cell (emitted by `iir-to-wasm` for
McCarthy Lisp) round-trips through the parser instead of being rejected.

- `parse_type_section` branches on the entry tag: `0x60` → function type (as
  before), `0x50` → a WasmGC **sub-type** entry, parsed as a struct type into
  the existing `WasmModule.struct_types`. Any other tag is a clean error.
- New `parse_struct_type` mirrors `wasm-module-encoder`'s `encode_struct_type`:
  `0x50 <supertype_count> 0x5F <field_count> [<val_type> <mutability>]*`. We
  require `supertype_count == 0` (explicit supertypes unsupported) and the
  struct composite marker `0x5F`.
- `decode_value_type` learned the single-byte GC reference types `anyref`
  (`0x6E`) and `i31ref` (`0x6C`); a new `read_value_type` streaming helper
  decodes the multi-byte concrete `structref` (`0x63 <typeidx>`) for struct
  field types.
- Function and struct types share one type-index space; since the encoder emits
  all function types first, a function's `module.types` index still equals its
  wasm type index (documented assumption).

Hardening: a vector length read from the (untrusted) byte stream is no longer
used directly as the pre-allocation capacity — `with_capacity` is capped at a
small bound and the `Vec` grows as elements actually arrive, so a crafted huge
`field_count`/`param_count` can't force a giant allocation. Truncated or
malformed struct types are clean errors, never panics.

8 new tests: struct-type recovery, func+struct mixed (index alignment),
immutable/`i31ref`/`structref` fields, and clean errors for a bad marker,
declared supertypes, truncation, and an unknown type tag.

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
