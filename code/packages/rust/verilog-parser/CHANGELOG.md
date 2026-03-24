# Changelog

All notable changes to the `coding-adventures-verilog-parser` crate will be documented in this file.

## [0.1.0] - 2026-03-22

### Added
- `create_verilog_parser(source)` -- factory function that loads `verilog.grammar` and returns a configured `GrammarParser`.
- `parse_verilog(source)` -- convenience function that parses Verilog source and returns a `GrammarASTNode`.
- Loads grammar from `verilog.grammar` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Depends on `verilog-lexer` for tokenization (including preprocessor support).
- Test suite covering empty modules, modules with ports, assign statements, always blocks, case statements, expressions, module instantiation, and the factory function.
