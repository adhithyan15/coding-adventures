# Changelog

All notable changes to the `coding-adventures-typescript-parser` crate will be documented in this file.

## [0.3.0] - 2026-05-21

### Removed
- Dropped support for the empty-string `""` "generic" version that pointed at the stub `code/grammars/typescript.grammar`. The full TS 1.0 through 5.8 grammars under `code/grammars/typescript/` supersede it.
- Removed the embedded `mod generic` block (~103 lines) from `_grammar.rs`.

### Changed
- Crate docstring no longer mentions the "generic" grammar.

### Migration
- Replace `parse_typescript(source, "")` with `parse_typescript(source, "ts5.8")` (or another explicit TS version).

### Notes
- Rust-only second step of CLOC01 Phase 1 stub retirement (sibling of PR #3785). Other language ports (Go, Python, TypeScript, Ruby) get equivalent follow-up PRs; the stub `.grammar` source file is preserved until all ports migrate.

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
