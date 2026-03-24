# Changelog

## [0.1.0] - 2026-03-22

### Added
- Initial release of the Go Verilog parser package.
- `ParseVerilog()` function that parses Verilog source code into generic `ASTNode` trees.
- `CreateVerilogParser()` factory function that tokenizes and configures the grammar-driven parser.
- Loads `verilog.grammar` from `code/grammars/`.
- Tests covering empty modules, modules with ports, assign statements, always blocks, case statements, and expressions.
