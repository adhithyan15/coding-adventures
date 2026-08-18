# Changelog

## 0.1.1 (2026-08-17)

### Fixed

- Eliminated runtime grammar loading: `create_css_lexer` now imports a
  pre-compiled `_grammar` module instead of reading and parsing the
  `.tokens` file from `code/grammars/` on every call. The old code walked
  out of the installed package's own directory to a monorepo-relative
  path that a published PyPI package does not ship, so `pip install` +
  first use would raise `FileNotFoundError`.

## 0.1.0 (2026-03-20)

- Initial release
- Thin wrapper around `GrammarLexer` for CSS tokenization
- Loads `css.tokens` grammar with 39 token definitions
- Supports compound tokens (DIMENSION, PERCENTAGE), function tokens,
  at-keywords, custom properties, vendor prefixes, unicode ranges
- Error token support (BAD_STRING, BAD_URL) for graceful degradation
- CSS escape sequences preserved raw (escapes: none mode)
