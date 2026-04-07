# Changelog

All notable changes to the `coding-adventures-typescript-lexer` crate will be documented in this file.

## [0.2.0] - 2026-04-05

### Changed
- `create_typescript_lexer(source, version)` now accepts a `version: &str` parameter and returns `Result<GrammarLexer, String>` instead of panicking.
- `tokenize_typescript(source, version)` now accepts a `version: &str` parameter and returns `Result<Vec<Token>, String>` instead of panicking.

### Added
- Version-aware grammar selection: pass `""` for the generic grammar or one of `"ts1.0"`, `"ts2.0"`, `"ts3.0"`, `"ts4.0"`, `"ts5.0"`, `"ts5.8"` for versioned TypeScript grammars stored in `grammars/typescript/`.
- `grammar_root()` helper that uses `PathBuf` navigation from `env!("CARGO_MANIFEST_DIR")` rather than string formatting.
- Returns `Err(String)` for unrecognised version strings instead of panicking on a missing file.
- New tests: `test_versioned_ts58`, `test_all_versioned_grammars`, `test_unknown_version_returns_err`, `test_create_lexer_unknown_version`.

## [0.1.0] - 2026-03-21

### Added
- `create_typescript_lexer(source)` — factory function that loads `typescript.tokens` and returns a configured `GrammarLexer`.
- `tokenize_typescript(source)` — convenience function that tokenizes TypeScript source and returns `Vec<Token>`.
- Loads grammar from `typescript.tokens` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Test suite covering typed declarations, keywords (including TypeScript-specific), arithmetic operators, multi-character operators, strings, numbers, comments, delimiters, whitespace, arrow operators, angle brackets, and the factory function.
