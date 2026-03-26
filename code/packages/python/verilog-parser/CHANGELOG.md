# Changelog

All notable changes to the Verilog Parser package will be documented in this file.

## [0.1.0] - 2026-03-22

### Added
- Initial release of the Verilog parser package.
- `parse_verilog()` function that parses Verilog HDL source code into generic `ASTNode` trees.
- `create_verilog_parser()` factory function for creating a `GrammarParser` configured for Verilog.
- Supports the synthesizable subset of IEEE 1364-2005 Verilog:
  - Module declarations with ports and parameters
  - Wire, reg, and integer declarations
  - Continuous assignments (`assign`)
  - Always blocks with sensitivity lists
  - If/else, case/casex/casez statements
  - Module instantiation with named and positional port connections
  - Generate blocks (for-generate, if-generate)
  - Full expression grammar with operator precedence
- Optional Verilog preprocessor integration (`preprocess=True` by default).
- Comprehensive test suite with 80%+ coverage.
