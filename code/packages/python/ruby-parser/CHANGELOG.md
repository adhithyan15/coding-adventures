# Changelog

All notable changes to the Ruby Parser package will be documented in this file.

## [0.1.0] - 2026-03-18

### Added
- Initial release of the Ruby parser package.
- `parse_ruby()` function that parses Ruby source code into a generic AST.
- `create_ruby_parser()` factory function for creating a `GrammarParser` configured for Ruby.
- Ruby parser grammar file (`ruby.grammar`) with support for:
  - Programs (sequences of statements)
  - Assignment statements (`x = expression`)
  - Method calls with arguments (`puts("hello")`)
  - Expression statements
  - Arithmetic expressions with operator precedence (`+`, `-`, `*`, `/`)
  - Parenthesized sub-expressions
  - Factors: number literals, string literals, variable names
- Comprehensive test suite with 80%+ coverage.
