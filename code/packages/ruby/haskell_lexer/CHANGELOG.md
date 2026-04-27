# Changelog

All notable changes to `coding_adventures_haskell_lexer` will be documented in this file.

## [0.1.0] - 2026-04-11

### Added
- Initial release
- `CodingAdventures::HaskellLexer.tokenize(source, version: nil)` method that tokenizes Haskell source code
- `CodingAdventures::HaskellLexer.create_lexer(source, version: nil)` factory method returning a configured lexer context
- `VALID_VERSIONS` constant listing all supported Haskell version strings: `"1.0"`, `"1.1"`, `"1.4"`, `"5"`, `"7"`, `"8"`, `"10"`, `"14"`, `"17"`, `"21"`
- `resolve_tokens_path(version)` class method -- resolves to `haskell/haskell<version>.tokens` (versioned) or `haskell/haskell21.tokens` (default)
- Raises `ArgumentError` with a descriptive message when an unknown version is given
- Loads `.tokens` grammar file and delegates to `GrammarLexer`
- Full test suite with SimpleCov coverage >= 80%
