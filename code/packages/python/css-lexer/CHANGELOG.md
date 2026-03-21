# Changelog

## 0.1.0 (2026-03-20)

- Initial release
- Thin wrapper around `GrammarLexer` for CSS tokenization
- Loads `css.tokens` grammar with 39 token definitions
- Supports compound tokens (DIMENSION, PERCENTAGE), function tokens,
  at-keywords, custom properties, vendor prefixes, unicode ranges
- Error token support (BAD_STRING, BAD_URL) for graceful degradation
- CSS escape sequences preserved raw (escapes: none mode)
