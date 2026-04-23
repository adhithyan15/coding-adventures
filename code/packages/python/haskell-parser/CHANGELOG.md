# Changelog

All notable changes to the Haskell Parser package will be documented in this file.

## [0.1.0] - 2026-04-11

### Added
- Initial release of the Haskell parser package.
- `parse_haskell()` function that parses Haskell source code into generic `ASTNode` trees.
- `create_haskell_parser()` factory function for creating a `GrammarParser` configured for Haskell.
- `version` parameter supporting Haskell versions: `"1.0"`, `"1.1"`, `"1.4"`, `"5"`, `"7"`,
  `"8"`, `"10"`, `"14"`, `"17"`, `"21"`. Default is `"21"` (latest).
- `_resolve_grammar_path(version)` private helper mapping version strings to grammar paths.
- Raises `ValueError` with a clear message for unknown version strings.
- Comprehensive test suite with 80%+ coverage.
