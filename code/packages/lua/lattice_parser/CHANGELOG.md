# Changelog — coding-adventures-lattice-parser

## 0.1.0 — Initial release

- Grammar-driven parser for the Lattice CSS superset language.
- Reads `code/grammars/lattice.grammar` at runtime (cached after first load).
- Delegates to `parser.GrammarParser` for the actual parse step.
- Public API: `parse(source)`, `create_parser(source)`, `get_grammar()`.
- Root AST node has `rule_name == "stylesheet"`.
- Supports all Lattice constructs: variables, mixins, @include, @if/@else,
  @for, @each, @while, @function/@return, @use, @extend, @content, @at-root,
  nested rules, placeholder selectors, map literals, and all CSS at-rules /
  qualified rules.
- Comprehensive busted test suite covering plain CSS, each Lattice construct,
  multi-rule stylesheets, create_parser, and error handling.
