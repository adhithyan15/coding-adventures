# Changelog

All notable changes to the Python Parser (TypeScript) package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the TypeScript Python parser package.
- `parsePython()` function that parses Python source code into generic `ASTNode` trees.
- Loads `python.grammar` file from `code/grammars/`.
- Delegates tokenization to `@coding-adventures/python-lexer`.
- Supports assignments, arithmetic expressions, operator precedence, and multiple statements.
- Comprehensive test suite with v8 coverage.
