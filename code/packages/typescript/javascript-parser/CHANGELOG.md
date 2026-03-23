# Changelog

All notable changes to the JavaScript Parser (TypeScript) package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the TypeScript JavaScript parser package.
- `parseJavascript()` function that parses JavaScript source code into generic `ASTNode` trees.
- Loads `javascript.grammar` file from `code/grammars/`.
- Delegates tokenization to `@coding-adventures/javascript-lexer`.
- Supports `var_declaration` (let/const/var), assignments, expression statements, and operator precedence.
- Comprehensive test suite with v8 coverage.
