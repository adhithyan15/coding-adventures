# Changelog

All notable changes to the `coding-adventures-javascript-lexer` crate will be documented in this file.

## [0.3.0] - 2026-05-20

### Removed
- Dropped support for the empty-string `""` "generic" version that pointed at the stub `code/grammars/javascript.tokens`. The stub was a 35-line proof-of-concept subset; the full ES1 through ES2025 grammars under `code/grammars/ecmascript/` supersede it.
- Removed the embedded `mod generic` block (~228 lines) from `_grammar.rs`.

### Changed
- `DEFAULT_VERSION` is now `"es2025"` (was `""`). Callers passing the old empty-string version now get `Err` with the supported-versions list.
- Crate docstring no longer mentions the "generic" grammar.

### Added
- `default_version_resolves_to_es2025` test verifies the new default.

### Migration
- Replace `tokenize_javascript(source, "")` with `tokenize_javascript(source, "es2025")` (or another explicit ES version).

### Notes
- This PR is the Rust-only first step of CLOC01 Phase 1 stub retirement. The stub `.tokens`/`.grammar` files remain on disk for now because the Go, Python, TypeScript, and Ruby ports still embed them. Those ports get equivalent follow-up PRs; once all are migrated, the stub source files will be deleted.

## [0.2.0] - 2026-04-05

### Changed
- `create_javascript_lexer(source, version)` now accepts a `version: &str` parameter and returns `Result<GrammarLexer, String>` instead of panicking.
- `tokenize_javascript(source, version)` now accepts a `version: &str` parameter and returns `Result<Vec<Token>, String>` instead of panicking.

### Added
- Version-aware grammar selection: pass `""` for the generic grammar or one of `"es1"`, `"es3"`, `"es5"`, `"es2015"`–`"es2025"` for versioned ECMAScript grammars stored in `grammars/ecmascript/`.
- `grammar_root()` helper that uses `PathBuf` navigation from `env!("CARGO_MANIFEST_DIR")` rather than string formatting.
- Returns `Err(String)` for unrecognised version strings instead of panicking on a missing file.
- New tests: `test_versioned_es2015`, `test_all_versioned_grammars`, `test_unknown_version_returns_err`, `test_create_lexer_unknown_version`.

## [0.1.0] - 2026-03-21

### Added
- `create_javascript_lexer(source)` — factory function that loads `javascript.tokens` and returns a configured `GrammarLexer`.
- `tokenize_javascript(source)` — convenience function that tokenizes JavaScript source and returns `Vec<Token>`.
- Loads grammar from `javascript.tokens` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Test suite covering variable declarations, keywords, arithmetic operators, multi-character operators, strings, numbers, comments, delimiters, whitespace, function expressions, arrow operators, and the factory function.
