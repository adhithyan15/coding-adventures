# Changelog

All notable changes to the Verilog Parser (TypeScript) package will be documented in this file.

## [0.1.0] - 2026-03-22

### Added
- Initial release of the TypeScript Verilog parser package.
- `parseVerilog()` function that parses Verilog (IEEE 1364-2005) source code into generic `ASTNode` trees.
- Loads `verilog.grammar` file from `code/grammars/`.
- Delegates tokenization to `@coding-adventures/verilog-lexer` (with preprocessor support).
- Supports module declarations, ports, wire/reg/integer declarations, continuous assignments, always blocks, initial blocks, if/else statements, case statements, module instantiation, and parameters.
- Full expression grammar with operator precedence (ternary, logical, bitwise, equality, relational, shift, arithmetic, unary).
- Comprehensive test suite with v8 coverage.
