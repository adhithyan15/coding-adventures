# Changelog

## [0.2.0] - 2026-07-13

### Changed

- The parser grammar is now embedded at compile time as native Go data in
  `grammar_data.go` and consumed directly by the loader; the package no longer
  reads `code/grammars/**` at run time via `runtime.Caller`. This drops the
  filesystem capability (empty `required_capabilities.json`, `gen_capabilities.go`
  removed), lets the package build and run standalone, and fixes the previously
  dead embedded grammar (the old `_grammar.go` name is ignored by the Go
  toolchain because of its leading underscore).

## [0.1.0] - 2026-03-22

### Added
- Initial release of the Go VHDL parser package.
- `ParseVhdl()` function that parses VHDL source code into generic `ASTNode` trees.
- `CreateVhdlParser()` factory function that tokenizes and configures the grammar-driven parser.
- Loads `vhdl.grammar` from `code/grammars/`.
- Tests covering empty entities, entities with ports, architectures, signal assignments, processes, if/elsif/else, and expressions.
