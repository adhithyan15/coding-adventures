# Changelog

All notable changes to the `coding-adventures-csharp-parser` crate will be documented in this file.

## [0.1.0] - 2026-04-11

### Added
- `create_csharp_parser(source, version)` — factory function that loads the appropriate `csharp{version}.grammar` and returns a configured `GrammarParser`. The `version` parameter selects the C# edition: `"1.0"`, `"2.0"`, `"3.0"`, `"4.0"`, `"5.0"`, `"6.0"`, `"7.0"`, `"8.0"`, `"9.0"`, `"10.0"`, `"11.0"`, `"12.0"` (default: `"12.0"`).
- `parse_csharp(source, version)` — convenience function that parses C# source and returns a `GrammarASTNode`.
- `grammar_root()` helper that uses `PathBuf` navigation from `env!("CARGO_MANIFEST_DIR")`.
- Returns `Err(String)` for unrecognised version strings instead of panicking on a missing file.
- The lexer is called with the same version string so tokens and grammar are always from the same C# edition — critical for version-sensitive keywords like `record` (C# 9.0+), `async`/`await` (C# 5.0+), and `dynamic` (C# 4.0+).
- Test suite (12 tests) covering class declarations, arithmetic expressions, multiple statements, empty programs, the factory function, versioned grammar selection for C# 8.0, 5.0, 3.0, and 12.0 individually, the all-versions smoke test (all 12 versions), and error cases for unknown versions and empty version strings.
