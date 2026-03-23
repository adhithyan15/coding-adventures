# Changelog

## [0.1.0] - 2026-03-22

### Added
- Initial release of the Go VHDL parser package.
- `ParseVhdl()` function that parses VHDL source code into generic `ASTNode` trees.
- `CreateVhdlParser()` factory function that tokenizes and configures the grammar-driven parser.
- Loads `vhdl.grammar` from `code/grammars/`.
- Tests covering empty entities, entities with ports, architectures, signal assignments, processes, if/elsif/else, and expressions.
