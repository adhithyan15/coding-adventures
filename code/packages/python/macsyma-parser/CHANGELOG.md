# Changelog

## 0.1.0 — 2026-04-19

Initial release.

- Thin wrapper around `GrammarParser`, configured via
  `code/grammars/macsyma/macsyma.grammar`.
- Parses the MACSYMA expression sublanguage: arithmetic, comparisons,
  boolean operators, function calls, lists, assignment, and function
  definition.
- Full test suite covering every production in the grammar.
