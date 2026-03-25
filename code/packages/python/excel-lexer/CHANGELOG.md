# Changelog

All notable changes to the JavaScript Lexer package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the JavaScript lexer package.
- `tokenize_excel()` function that tokenizes JavaScript source code using the grammar-driven lexer.
- `create_excel_lexer()` factory function for creating a `GrammarLexer` configured for JavaScript.
- JavaScript token grammar file (`excel.tokens`) with support for:
  - JavaScript keywords: `let`, `const`, `var`, `function`, `if`, `else`, `while`, `for`, `return`, `class`, `true`, `false`, `null`, `undefined`, etc.
  - JavaScript-specific operators: `===`, `!==`, `=>`, `==`, `!=`, `<=`, `>=`
  - Delimiters: `()`, `{}`, `[]`, `;`, `,`, `:`, `.`
  - `$` in identifiers
- Comprehensive test suite with 80%+ coverage.
