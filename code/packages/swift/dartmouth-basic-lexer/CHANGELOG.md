# Changelog -- DartmouthBasicLexer (Swift)

## [0.2.0] -- 2026-07-13

### Changed
- `loadGrammar()` now returns the grammar embedded at compile time in the
  generated `_Grammar.swift` instead of reading `code/grammars/**` via
  `#filePath` at run time. The lexer/parser no longer depends on the
  monorepo layout and works when published standalone. Grammar source of
  truth is unchanged; regenerate via `swift/grammar-tools`' `grammar-tools-embed`.

## [0.1.2] -- 2026-07-12

### Fixed
- `loadGrammar()` now reads the grammar from
  `grammars/dartmouth_basic/dartmouth_basic.tokens`. PR #7475 moved every
  grammar into a per-grammar `code/grammars/<name>/` subdirectory, but this
  lexer still resolved the old flat `grammars/dartmouth_basic.tokens` path,
  so every test failed with "The file doesn't exist."

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
