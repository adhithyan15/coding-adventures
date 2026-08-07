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

## [0.1.0] - 2026-03-19

### Added
- Initial release of the Go JavaScript parser package.
- `ParseExcel()` function that parses JavaScript source code into generic `ASTNode` trees.
- `CreateExcelParser()` factory function.
- Loads `excel.grammar` from `code/grammars/`.
