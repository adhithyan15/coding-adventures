# Changelog -- DartmouthBasicLexer (Swift)

## [0.1.1] -- 2026-04-10

### Added
- `promoteKeywords(_:)` -- post-processing Pass 0 that converts NAME tokens whose
  uppercased value is a BASIC keyword into KEYWORD tokens with the uppercase value.
  Added as a safety net: the GrammarLexer's own keyword-promotion fires when
  `@case_insensitive true` is set (added back to `dartmouth_basic.tokens`), but
  this pass ensures correctness even if keyword lookup ever misses.

## [0.1.0] -- 2026-04-10

### Added

- Initial implementation of `DartmouthBasicLexer`.
- `tokenize(_:)` -- tokenizes Dartmouth BASIC (1964) source using `dartmouth_basic.tokens`.
- `loadGrammar()` -- loads and parses the Dartmouth BASIC token grammar.
- `relabelLineNumbers(_:)` -- post-processing pass that promotes the first NUMBER
  on each line to `LINE_NUM`.
- `suppressRemContent(_:)` -- post-processing pass that removes comment tokens
  between a `REM` keyword and the following `NEWLINE`.
- Full Dartmouth BASIC 1964 token set: 20 keywords, 11 built-in functions,
  user-defined function names (FNA–FNZ), numeric literals (integer, decimal,
  scientific notation), string literals, variables (A–Z, A0–Z9), and operators.
- Comprehensive XCTest suite covering all token types, post-processing passes,
  position tracking, and edge cases.
- `BUILD` and `BUILD_windows` scripts.
- `.gitignore` with `.build/`.
- `required_capabilities.json` declaring `filesystem:read`.
