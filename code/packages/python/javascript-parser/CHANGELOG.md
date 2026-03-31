# Changelog

All notable changes to the JavaScript Parser package will be documented in this file.

## [0.1.1] - 2026-03-31

### Fixed

- Updated `javascript.grammar` so that `var_declaration` and `factor` reference
  `KEYWORD` instead of `VAR | LET | CONST` and `TRUE | FALSE | NULL | UNDEFINED`.
  The grammar-driven lexer reclassifies all keyword identifiers to `KEYWORD` tokens;
  the grammar must use that token type rather than individual keyword names.
  This fixes `GrammarParseError: Parse error at 1:1: Unexpected token: 'let'` and
  the same error for `const`.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the JavaScript parser package.
- `parse_javascript()` function that parses JavaScript source code into generic `ASTNode` trees.
- `create_javascript_parser()` factory function for creating a `GrammarParser` configured for JavaScript.
- Supports `var_declaration` (let/const/var), assignments, expression statements, and operator precedence.
- Comprehensive test suite with 80%+ coverage.
