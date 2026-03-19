# Changelog

All notable changes to the JavaScript Lexer (TypeScript) package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the TypeScript JavaScript lexer package.
- `tokenizeJavascript()` function that tokenizes JavaScript source code using the grammar-driven lexer.
- Loads `javascript.tokens` grammar file from `code/grammars/`.
- Supports JavaScript keywords: `let`, `const`, `var`, `function`, `if`, `else`, `while`, `for`, `return`, `class`, `true`, `false`, `null`, `undefined`, etc.
- Supports JavaScript-specific operators: `===`, `!==`, `=>`, `==`, `!=`, `<=`, `>=`.
- Supports delimiters: `()`, `{}`, `[]`, `;`, `,`, `:`, `.`.
- Supports `$` in identifiers.
- Comprehensive test suite with v8 coverage.
