# Changelog

All notable changes to the JavaScript Parser package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the JavaScript parser package.
- `parse_javascript()` function that parses JavaScript source code into generic `ASTNode` trees.
- `create_javascript_parser()` factory function for creating a `GrammarParser` configured for JavaScript.
- Supports `var_declaration` (let/const/var), assignments, expression statements, and operator precedence.
- Comprehensive test suite with 80%+ coverage.
