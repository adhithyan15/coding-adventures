# Changelog — CodingAdventures::LatticeParser

## 0.01 — Initial release

- Hand-written recursive-descent parser for the Lattice CSS superset language.
- Parses all Lattice constructs: variables, mixins, @include, @if/@else,
  @for (through/to), @each, @while, @function/@return, @use, @extend,
  @content, @at-root, map literals, and all standard CSS at-rules and
  qualified rules.
- Full CSS support: type/class/id/attribute/pseudo-class/pseudo-element
  selectors, combinators, declaration blocks, nested rules, !important.
- ASTNode submodule (`CodingAdventures::LatticeParser::ASTNode`) mirrors
  the structure of TomlParser::ASTNode for stack consistency.
- Test suite (`t/00-load.t`, `t/01-basic.t`) covers all major constructs,
  ASTNode accessors, and error handling paths.
- Depends only on `CodingAdventures::LatticeLexer` at runtime.
