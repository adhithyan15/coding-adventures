# Changelog

## [0.1.0] - Unreleased

### Added

- Added the Elixir Python parser wrapper over the shared grammar-driven parser.
- Added version-aware lexing for Python 2.7, 3.0, 3.6, 3.8, 3.10, and 3.12 before parsing with the shared Python subset grammar.
- Added tests for assignments, arithmetic precedence, version selection, parser caching, lexer errors, and malformed syntax.
