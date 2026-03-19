# Changelog

All notable changes to the Ruby Lexer (TypeScript) package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the TypeScript Ruby lexer package.
- `tokenizeRuby()` function that tokenizes Ruby source code using the grammar-driven lexer.
- Loads `ruby.tokens` grammar file from `code/grammars/`.
- Supports Ruby keywords, operators (`..`, `=>`, `!=`, `<=`, `>=`), strings, and numbers.
- Comprehensive test suite with v8 coverage.
