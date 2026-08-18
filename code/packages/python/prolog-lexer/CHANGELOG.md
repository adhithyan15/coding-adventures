# Changelog

All notable changes to this package will be documented in this file.

## [0.1.1] - 2026-08-17

### Fixed

- Eliminated runtime grammar loading: `create_prolog_lexer` now imports a
  pre-compiled `_grammar` module instead of reading and parsing the
  `.tokens` file from `code/grammars/` on every call. The old code walked
  out of the installed package's own directory to a monorepo-relative
  path that a published PyPI package does not ship, so `pip install` +
  first use would raise `FileNotFoundError`.

## [0.1.0] - 2026-04-18

### Added

- grammar-driven Prolog lexer wrapper
- `code/grammars/prolog.tokens` for atoms, variables, numbers, strings, rules, queries, and list punctuation
- pytest coverage for facts, rules, queries, literals, comments, and source positions
