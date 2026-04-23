# Changelog

All notable changes to the `coding-adventures-java-parser` crate will be documented in this file.

## [0.1.0] - 2026-04-11

### Added
- `create_java_parser(source, version)` — factory function that loads the appropriate `java{version}.grammar` and returns a configured `GrammarParser`. The `version` parameter selects the Java edition: `"1.0"`, `"1.1"`, `"1.4"`, `"5"`, `"7"`, `"8"`, `"10"`, `"14"`, `"17"`, `"21"` (default: `"21"`).
- `parse_java(source, version)` — convenience function that parses Java source and returns a `GrammarASTNode`.
- `grammar_root()` helper that uses `PathBuf` navigation from `env!("CARGO_MANIFEST_DIR")`.
- Returns `Err(String)` for unrecognised version strings instead of panicking on a missing file.
- The lexer is called with the same version string so tokens and grammar are always from the same Java edition.
- Test suite covering class declarations, expressions, multiple statements, empty programs, factory function, versioned grammar selection, and error cases for unknown versions.
