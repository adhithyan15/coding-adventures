# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- Initial implementation of `WasmModuleParser` class with `parse(data: Uint8Array): WasmModule`
- `WasmParseError` class with `offset` field for precise error reporting
- Full WASM 1.0 binary format support:
  - Header validation (magic bytes `\0asm` + version `[1,0,0,0]`)
  - Type section (ID 1): function signatures with param/result ValueType vectors
  - Import section (ID 2): function, table, memory, and global imports
  - Function section (ID 3): type-index mapping for local functions
  - Table section (ID 4): funcref tables with Limits
  - Memory section (ID 5): linear memory declarations with Limits
  - Global section (ID 6): module globals with init expressions
  - Export section (ID 7): function, table, memory, global exports
  - Start section (ID 8): module entry-point function index
  - Element section (ID 9): table initializer segments
  - Code section (ID 10): function bodies with run-length-expanded locals
  - Data section (ID 11): linear memory initializer segments
  - Custom sections (ID 0): named byte blobs, anywhere in file
- Section ordering enforcement (numbered sections 1–11 must be ascending)
- Custom sections exempt from ordering requirement
- Unknown section IDs skipped silently (forward-compatibility)
- ULEB128 reading delegated to `@coding-adventures/wasm-leb128`
- Structured output via types from `@coding-adventures/wasm-types`
- 49 unit tests covering all sections, error paths, and round-trip verification
- 90%+ test coverage
