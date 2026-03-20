# Changelog

All notable changes to the Starlark Lexer (TypeScript) package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the TypeScript Starlark lexer package.
- `tokenizeStarlark()` function that tokenizes Starlark source code using the grammar-driven lexer.
- Loads `starlark.tokens` grammar file from `code/grammars/`.
- Supports Starlark keywords (`def`, `return`, `if`, `else`, `elif`, `for`, `in`, `pass`, etc.).
- Reserved keyword detection (`class`, `import`, `while`, `try`, etc.) with clear error messages.
- Indentation mode for INDENT/DEDENT/NEWLINE token emission.
- Multi-character operators (`**`, `//`, `==`, `!=`, `+=`, `-=`, `//=`, etc.).
- String literals (single, double, triple-quoted, raw, bytes).
- Comment skipping.
- Comprehensive test suite with v8 coverage.
