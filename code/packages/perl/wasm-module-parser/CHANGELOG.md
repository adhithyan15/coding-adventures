# Changelog

## [0.01] - 2026-03-29

### Added

- Initial implementation of `CodingAdventures::WasmModuleParser`.
- `parse($bytes_str)` — parse a complete WebAssembly binary module from a
  binary string.
- `parse_header(\@bytes, $offset)` — validate the 8-byte Wasm header
  (magic `\x00asm` + version 1).
- `parse_section(\@bytes, $offset)` — parse one section envelope (ID + LEB128
  length); returns section info hashref and content start offset.
- `get_section(\%module, $section_id)` — retrieve a parsed section from a
  module hashref by section ID constant.
- Full parsing support for all standard Wasm sections:
  - Type section (function signatures with params/results)
  - Import section (function, table, memory, and global imports)
  - Function section (type index array)
  - Table section (reference tables with ref type + limits)
  - Memory section (linear memory limits)
  - Global section (val type + mutability + init expression)
  - Export section (function, table, memory, global exports)
  - Start section (start function index)
  - Code section (function bodies with local variable groups + raw bytecode)
  - Custom section (arbitrary name + byte content)
  - Element and Data sections preserved as raw byte arrays
- Section ID constants: `SECTION_CUSTOM` (0) through `SECTION_DATA` (11).
- Module-level constants: `MODULE_MAGIC` and `MODULE_VERSION`.
- Depends on `CodingAdventures::WasmLeb128` and `CodingAdventures::WasmTypes`.
- Test suite covering all section types, error handling, and the combined
  multi-section case.
