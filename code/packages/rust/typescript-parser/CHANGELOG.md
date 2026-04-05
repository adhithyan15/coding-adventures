# Changelog

All notable changes to the `coding-adventures-typescript-parser` crate will be documented in this file.

## [0.2.0] - 2026-04-05

### Changed
- `create_typescript_parser(source, version)` now accepts a `version: &str` parameter and returns `Result<GrammarParser, String>` instead of panicking.
- `parse_typescript(source, version)` now accepts a `version: &str` parameter and returns `Result<GrammarASTNode, String>` instead of panicking.

### Added
- Version-aware grammar selection: pass `""` for the generic grammar or one of `"ts1.0"`, `"ts2.0"`, `"ts3.0"`, `"ts4.0"`, `"ts5.0"`, `"ts5.8"` for versioned TypeScript grammars stored in `grammars/typescript/`.
- `grammar_root()` helper that uses `PathBuf` navigation from `env!("CARGO_MANIFEST_DIR")`.
- Returns `Err(String)` for unrecognised version strings instead of panicking on a missing file.
- The lexer is called with the same version string so tokens and grammar are always from the same TypeScript edition.
- New tests: `test_versioned_ts58`, `test_all_versioned_grammars`, `test_unknown_version_returns_err`, `test_create_parser_unknown_version`.

## [0.1.0] - 2026-03-21

### Added
- `create_typescript_parser(source)` — factory function that loads `typescript.grammar` and returns a configured `GrammarParser`.
- `parse_typescript(source)` — convenience function that parses TypeScript source and returns a `GrammarASTNode`.
- Loads grammar from `typescript.grammar` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Test suite covering typed declarations, expressions, function declarations, if/else, while loops, for loops, multiple statements, empty programs, interface declarations, and the factory function.
