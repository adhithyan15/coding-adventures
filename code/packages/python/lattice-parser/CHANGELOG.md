# Changelog

## [0.1.0] - 2026-03-22

### Added

- Thin wrapper around `GrammarParser` for Lattice parsing
- Loads `lattice.grammar` with EBNF rules for the Lattice CSS superset
- Parses Lattice-specific constructs: variable declarations, mixin definitions,
  function definitions, `@include`, `@if`/`@else`, `@for`, `@each`, `@return`
- Parses full CSS3 constructs: qualified rules, at-rules, selectors, declarations
- `parse_lattice()` convenience function returning an `ASTNode` tree
- `create_lattice_parser()` for lower-level access to the `GrammarParser` instance
