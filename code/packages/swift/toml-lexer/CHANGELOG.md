# Changelog -- TOMLLexer (Swift)

## [0.1.0] -- 2026-04-12

### Added

- Initial implementation of `TOMLLexer`.
- `tokenize(_:)` -- tokenizes TOML source using `toml.tokens`.
- `loadGrammar()` -- loads and parses the TOML token grammar.
- Comprehensive XCTest suite covering TOML complexities (strings, dates, numbers, keys).
- `BUILD` and `BUILD_windows` scripts.
