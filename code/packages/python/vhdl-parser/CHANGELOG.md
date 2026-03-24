# Changelog

All notable changes to the VHDL Parser package will be documented in this file.

## [0.1.0] - 2026-03-22

### Added
- Initial release of the VHDL parser package.
- `parse_vhdl()` function that parses VHDL source code into generic `ASTNode` trees.
- `create_vhdl_parser()` factory function for creating a `GrammarParser` configured for VHDL.
- Supports the synthesizable subset of IEEE 1076-2008 VHDL:
  - Entity declarations with ports and generics
  - Architecture bodies with signal declarations
  - Concurrent signal assignments
  - Process statements with sensitivity lists
  - If/elsif/else statements
  - Case/when statements with choices
  - Component instantiation with port maps
  - Variable assignment (`:=`) and signal assignment (`<=`)
  - Full expression grammar with keyword operators (and, or, xor, not, etc.)
- Case-insensitive parsing via the VHDL lexer's normalization.
- Comprehensive test suite with 80%+ coverage.
