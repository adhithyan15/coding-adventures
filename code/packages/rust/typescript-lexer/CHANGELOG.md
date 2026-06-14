# Changelog

All notable changes to the `coding-adventures-typescript-lexer` crate will be documented in this file.

## [0.3.0] - 2026-05-21

### Removed
- Dropped support for the empty-string `""` "generic" version that pointed at the stub `code/grammars/typescript.tokens`. The full TS 1.0 through 5.8 grammars under `code/grammars/typescript/` supersede it.
- Removed the embedded `mod generic` block (~228 lines) from `_grammar.rs`.

### Changed
- `DEFAULT_VERSION` is now `"ts5.8"` (was `""`). Callers passing the old empty-string version now get `Err` with the supported-versions list.
- Crate docstring no longer mentions the "generic" grammar.

### Added
- `default_version_resolves_to_ts5_8` test verifies the new default.

### Migration
- Replace `tokenize_typescript(source, "")` with `tokenize_typescript(source, "ts5.8")` (or another explicit TS version).

### Notes
- This PR is the Rust-only second step of CLOC01 Phase 1 stub retirement (sibling of the JS port retirement, PR #3785). The stub `.tokens`/`.grammar` files remain on disk for now because the Go, Python, TypeScript, and Ruby ports still embed them. Those ports get equivalent follow-up PRs.

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
