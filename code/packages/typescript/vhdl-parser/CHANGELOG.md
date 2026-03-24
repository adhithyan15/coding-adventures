# Changelog

All notable changes to the VHDL Parser (TypeScript) package will be documented in this file.

## [0.1.0] - 2026-03-22

### Added
- Initial release of the TypeScript VHDL parser package.
- `parseVhdl()` function that parses VHDL (IEEE 1076-2008) source code into generic `ASTNode` trees.
- Loads `vhdl.grammar` file from `code/grammars/`.
- Delegates tokenization to `@coding-adventures/vhdl-lexer` (with case normalization).
- Supports entity declarations, architecture bodies, signal declarations, constant declarations, type declarations (enumerations), port clauses, generic clauses, concurrent signal assignments, process statements, if/elsif/else statements, case/when statements, component instantiation, and library/use clauses.
- Full expression grammar with operator precedence (logical, relational, shift, adding, multiplying, unary, power).
- Comprehensive test suite with v8 coverage.
