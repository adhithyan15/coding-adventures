# Changelog — coding-adventures-css-lexer (Lua)

## [0.1.0] — Initial release

### Added
- `tokenize(source)` — tokenizes a CSS3 source string into a flat token list
- `get_grammar()` — returns the cached TokenGrammar for inspection or reuse
- Grammar loaded from `code/grammars/css.tokens` (6-level path navigation)
- Grammar caching — css.tokens is read and parsed only once per process
- Support for all CSS3 token types:
  - Compound tokens: DIMENSION, PERCENTAGE, AT_KEYWORD, HASH, FUNCTION, URL_TOKEN
  - String literals: STRING (both double-quoted and single-quoted via -> alias)
  - Identifiers: IDENT, CUSTOM_PROPERTY, UNICODE_RANGE
  - All delimiters and operators including COLON_COLON, TILDE_EQUALS, etc.
  - Error tokens: BAD_STRING, BAD_URL for graceful degradation
- `escapes: none` mode — CSS hex escapes preserved as raw text
- Skip patterns for whitespace and `/* ... */` comments
- First-match-wins priority ordering guarantees:
  - DIMENSION wins over NUMBER + IDENT (10px is one token)
  - PERCENTAGE wins over NUMBER (50% is one token)
  - FUNCTION wins over IDENT (rgba( is one token)
  - COLON_COLON wins over COLON (:: is one token)
  - CUSTOM_PROPERTY wins over IDENT (--var is one token)
- Comprehensive busted test suite in `tests/test_css_lexer.lua`
