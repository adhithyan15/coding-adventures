# Changelog

All notable changes to `coding_adventures_haskell_parser` will be documented in this file.

## [0.1.0] - 2026-04-11

### Added
- Initial release
- `CodingAdventures::HaskellParser.parse(source, version: nil)` method that parses Haskell source code into ASTs
- `CodingAdventures::HaskellParser.create_parser(source, version: nil)` factory method returning a configured parser context
- `VALID_VERSIONS` constant listing all supported Haskell version strings: `"1.0"`, `"1.1"`, `"1.4"`, `"5"`, `"7"`, `"8"`, `"10"`, `"14"`, `"17"`, `"21"`
- `resolve_grammar_path(version)` class method -- resolves to `haskell/haskell<version>.grammar` (versioned) or `haskell/haskell21.grammar` (default)
- Loads `haskell/haskell<version>.grammar` grammar file and delegates to `GrammarDrivenParser`
- Raises `ArgumentError` with a descriptive message when an unknown version is given
- Full test suite with SimpleCov coverage >= 80%
