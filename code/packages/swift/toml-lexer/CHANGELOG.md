# Changelog -- TOMLLexer (Swift)

## [0.1.1] -- 2026-07-12

### Fixed
- `loadGrammar()` now reads the grammar from `grammars/toml/toml.tokens`.
  PR #7475 moved every grammar into a per-grammar `code/grammars/<name>/`
  subdirectory; this lexer still used the old flat `grammars/toml.tokens`
  path, so every test failed with "The file doesn't exist."

## [0.1.0] -- 2026-04-12

### Added

- Initial implementation of `TOMLLexer`.
- `tokenize(_:)` -- tokenizes TOML source using `toml.tokens`.
- `loadGrammar()` -- loads and parses the TOML token grammar.
- Comprehensive XCTest suite covering TOML complexities (strings, dates, numbers, keys).
- `BUILD` and `BUILD_windows` scripts.
