# Changelog

All notable changes to the `coding-adventures-vhdl-parser` crate will be documented in this file.

## [0.1.0] - 2026-03-22

### Added
- `create_vhdl_parser(source)` -- factory function that loads `vhdl.grammar` and returns a configured `GrammarParser`.
- `parse_vhdl(source)` -- convenience function that parses VHDL source and returns a `GrammarASTNode`.
- Loads grammar from `vhdl.grammar` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Depends on `vhdl-lexer` for tokenization (case-insensitive, no preprocessor).
- Test suite covering empty entities, entities with ports, architectures, signal assignments, processes, if/else statements, case statements, and expressions.
