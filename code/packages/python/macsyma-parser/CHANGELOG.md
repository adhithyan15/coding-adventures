# Changelog

## 0.1.1 — 2026-08-17

### Fixed

- Eliminated runtime grammar loading: `create_macsyma_parser` now imports
  a pre-compiled `_grammar` module instead of reading and parsing the
  `.grammar` file from `code/grammars/` on every call. The old code walked
  out of the installed package's own directory to a monorepo-relative
  path that a published PyPI package does not ship, so `pip install` +
  first use would raise `FileNotFoundError`.

## 0.1.0 — 2026-04-19

Initial release.

- Thin wrapper around `GrammarParser`, configured via
  `code/grammars/macsyma/macsyma.grammar`.
- Parses the MACSYMA expression sublanguage: arithmetic, comparisons,
  boolean operators, function calls, lists, assignment, and function
  definition.
- Full test suite covering every production in the grammar.
