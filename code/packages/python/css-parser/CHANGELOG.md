# Changelog

## 0.1.1 (2026-08-17)

### Fixed

- Eliminated runtime grammar loading: `create_css_parser` now imports a
  pre-compiled `_grammar` module instead of reading and parsing the
  `.grammar` file from `code/grammars/` on every call. The old code walked
  out of the installed package's own directory to a monorepo-relative
  path that a published PyPI package does not ship, so `pip install` +
  first use would raise `FileNotFoundError`.

## 0.1.0 (2026-03-20)

- Initial release
- Thin wrapper around `GrammarParser` for CSS parsing
- Loads `css.grammar` with 36 grammar rules
- Supports full CSS3 selector syntax (type, class, ID, attribute,
  pseudo-class, pseudo-element, combinators, nesting)
- At-rule parsing (@media, @import, @keyframes, @font-face, etc.)
- Declaration blocks with `!important` priority annotation
- CSS values: dimensions, percentages, colors, functions, custom properties
- CSS Nesting support with `&` ampersand references
- Nested function calls (calc, var, rgb, etc.)
