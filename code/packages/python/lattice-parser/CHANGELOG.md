# Changelog

## [0.1.1] - 2026-08-17

### Fixed

- Eliminated runtime grammar loading: `create_lattice_parser` now imports a pre-compiled `_grammar` module instead of reading and parsing the `lattice.grammar` file from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published PyPI package does not ship, so `pip install` + first use would raise `FileNotFoundError`.
- The pre-existing checked-in `src/lattice_parser/_grammar.py` predated a `grammar-tools` compiler update (missing the `# ruff: noqa` header the compiler now emits) and was never imported by `parser.py`, which always read `lattice.grammar` directly from disk. It has been regenerated fresh from the current grammar file and wired in.

## [0.1.0] - 2026-03-22

### Added

- Thin wrapper around `GrammarParser` for Lattice parsing
- Loads `lattice.grammar` with EBNF rules for the Lattice CSS superset
- Parses Lattice-specific constructs: variable declarations, mixin definitions,
  function definitions, `@include`, `@if`/`@else`, `@for`, `@each`, `@return`
- Parses full CSS3 constructs: qualified rules, at-rules, selectors, declarations
- `parse_lattice()` convenience function returning an `ASTNode` tree
- `create_lattice_parser()` for lower-level access to the `GrammarParser` instance
