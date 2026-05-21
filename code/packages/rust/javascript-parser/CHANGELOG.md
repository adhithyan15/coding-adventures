# Changelog

All notable changes to the `coding-adventures-javascript-parser` crate will be documented in this file.

## [0.4.0] - 2026-05-21

### Added
- New dependency on `coding-adventures-javascript-tokens` for the shared `EsVersion` enum.
- `create_javascript_parser_typed(source, EsVersion) -> Result<GrammarParser, String>` — typed constructor; no unknown-version error path.
- `parse_javascript_typed(source, EsVersion) -> Result<GrammarASTNode, String>` — typed parser.
- `pub const DEFAULT_ES_VERSION: EsVersion = EsVersion::Es2025;` — typed default.
- New tests covering the typed APIs: `parse_typed_es2015`, `default_es_version_constant_is_es2025`, `all_typed_versions_load`, `create_parser_typed`.

### Notes
- The existing `&str`-based APIs are kept for backwards compatibility. Typed APIs are the preferred surface going forward.
- The typed parser delegates to `javascript-lexer`'s `tokenize_javascript_typed`, so token/grammar versions are guaranteed to come from the same ECMAScript edition.

## [0.3.0] - 2026-05-20

### Removed
- Dropped support for the empty-string `""` "generic" version that pointed at the stub `code/grammars/javascript.grammar`. The full ES1 through ES2025 grammars under `code/grammars/ecmascript/` supersede it.
- Removed the embedded `mod generic` block (~103 lines) from `_grammar.rs`.

### Changed
- Crate docstring no longer mentions the "generic" grammar.

### Migration
- Replace `parse_javascript(source, "")` with `parse_javascript(source, "es2025")` (or another explicit ES version).

### Notes
- Rust-only first step of CLOC01 Phase 1 stub retirement. Other language ports (Go, Python, TypeScript, Ruby) get equivalent follow-up PRs; the stub `.grammar` source file is preserved until all ports migrate.

## [0.2.0] - 2026-04-05

### Changed
- `create_javascript_parser(source, version)` now accepts a `version: &str` parameter and returns `Result<GrammarParser, String>` instead of panicking.
- `parse_javascript(source, version)` now accepts a `version: &str` parameter and returns `Result<GrammarASTNode, String>` instead of panicking.

### Added
- Version-aware grammar selection: pass `""` for the generic grammar or one of `"es1"`, `"es3"`, `"es5"`, `"es2015"`–`"es2025"` for versioned ECMAScript grammars stored in `grammars/ecmascript/`.
- `grammar_root()` helper that uses `PathBuf` navigation from `env!("CARGO_MANIFEST_DIR")`.
- Returns `Err(String)` for unrecognised version strings instead of panicking on a missing file.
- The lexer is called with the same version string so tokens and grammar are always from the same ECMAScript edition.
- New tests: `test_versioned_es2015`, `test_all_versioned_grammars`, `test_unknown_version_returns_err`, `test_create_parser_unknown_version`.

## [0.1.0] - 2026-03-21

### Added
- `create_javascript_parser(source)` — factory function that loads `javascript.grammar` and returns a configured `GrammarParser`.
- `parse_javascript(source)` — convenience function that parses JavaScript source and returns a `GrammarASTNode`.
- Loads grammar from `javascript.grammar` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Test suite covering variable declarations, expressions, function declarations, if/else, while loops, for loops, multiple statements, empty programs, function calls, and the factory function.
