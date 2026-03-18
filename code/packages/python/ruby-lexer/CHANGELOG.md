# Changelog

All notable changes to the Ruby Lexer package will be documented in this file.

## [0.1.0] - 2026-03-18

### Added
- Initial release of the Ruby lexer package.
- `tokenize_ruby()` function that tokenizes Ruby source code using the grammar-driven lexer.
- `create_ruby_lexer()` factory function for creating a `GrammarLexer` configured for Ruby.
- Ruby token grammar file (`ruby.tokens`) with support for:
  - Ruby keywords: `def`, `end`, `if`, `else`, `elsif`, `puts`, `true`, `false`, `nil`, etc.
  - Ruby-specific operators: `..` (range), `=>` (hash rocket), `!=`, `<=`, `>=`
  - Standard operators: `+`, `-`, `*`, `/`, `=`, `==`
  - Literals: names, numbers, double-quoted strings with escape sequences
  - Delimiters: parentheses, commas, colons
- Comprehensive test suite with 80%+ coverage.
