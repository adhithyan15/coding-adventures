# Changelog

## [0.1.0] - 2026-03-29

### Added

- Initial implementation of `wasm_module_parser` Lua package.
- `M.parse(bytes)` — parse a complete WebAssembly binary module from a binary string.
- `M.parse_header(bytes, pos)` — validate the 8-byte Wasm header (magic + version).
- `M.parse_section(bytes, pos)` — parse one section envelope (ID + LEB128 length).
- `M.get_section(module, section_id)` — retrieve a parsed section from a module.
- Full parsing support for:
  - Type section (function signatures)
  - Import section (functions, tables, memories, globals)
  - Function section (type indices)
  - Table section (reference tables)
  - Memory section (linear memory limits)
  - Global section (global variables with init expressions)
  - Export section (exported functions, tables, memories, globals)
  - Start section (start function index)
  - Code section (function bodies with local variable groups)
  - Custom section (arbitrary name + byte content)
  - Element and Data sections preserved as raw bytes
- Section ID constants: `SECTION_CUSTOM` through `SECTION_DATA`.
- Depends on `coding-adventures-wasm-leb128` and `coding-adventures-wasm-types`.
- Comprehensive test suite with hand-crafted Wasm binaries covering all section types.
