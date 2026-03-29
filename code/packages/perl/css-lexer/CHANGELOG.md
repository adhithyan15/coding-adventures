# Changelog — CodingAdventures::CssLexer (Perl)

## [0.01] — Initial release

### Added
- `tokenize($source)` — tokenizes a CSS3 source string into a flat list of token hashrefs
- Each token hashref has keys: `type`, `value`, `line`, `col`
- Grammar loaded from `code/grammars/css.tokens` (5-level path navigation via File::Spec)
- Grammar caching — css.tokens is read and parsed only once per process
- `escapes: none` mode — CSS hex escapes preserved as raw text in token values
- Perl `\G` + `pos()` mechanism for anchored pattern matching
- `/s` modifier on skip patterns to match `/* ... */` comments across newlines
- All CSS3 token types:
  - Compound tokens: DIMENSION, PERCENTAGE, AT_KEYWORD, HASH, FUNCTION, URL_TOKEN
  - String literals: STRING (double-quoted and single-quoted, both emit STRING via alias)
  - Identifiers: IDENT, CUSTOM_PROPERTY, UNICODE_RANGE
  - Legacy HTML comment delimiters: CDO, CDC
  - All delimiters and operators including COLON_COLON, TILDE_EQUALS, etc.
  - Error tokens: BAD_STRING, BAD_URL for graceful degradation
- Skip patterns for whitespace and `/* ... */` CSS comments
- First-match-wins priority ordering (preserved from css.tokens definition order):
  - DIMENSION wins over NUMBER + IDENT (10px is one token)
  - PERCENTAGE wins over NUMBER (50% is one token)
  - URL_TOKEN wins over FUNCTION (url(./x) is one token)
  - FUNCTION wins over IDENT (rgba( is one token)
  - COLON_COLON wins over COLON (:: is one token)
  - CUSTOM_PROPERTY wins over IDENT (--var is one token)
- Comprehensive Test2::V0 test suite in `t/00-load.t` and `t/01-basic.t`
