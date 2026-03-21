# Changelog

All notable changes to the Python Lexer (TypeScript) package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the TypeScript Python lexer package.
- `tokenizePython()` function that tokenizes Python source code using the grammar-driven lexer.
- Loads `python.tokens` grammar file from `code/grammars/`.
- Supports Python keywords: `if`, `else`, `elif`, `while`, `for`, `def`, `return`, `class`, `import`, `from`, `as`, `True`, `False`, `None`.
- Supports operators: `+`, `-`, `*`, `/`, `=`, `==`.
- Supports delimiters: `(`, `)`, `,`, `:`.
- Supports string literals, numeric literals, and identifiers.
- Comprehensive test suite with v8 coverage.
