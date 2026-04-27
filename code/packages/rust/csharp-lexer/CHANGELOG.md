# Changelog

All notable changes to the `coding-adventures-csharp-lexer` crate will be documented in this file.

## [0.1.0] - 2026-04-11

### Added
- `create_csharp_lexer(source, version)` — factory function that loads the appropriate `csharp{version}.tokens` grammar and returns a configured `GrammarLexer`. The `version` parameter selects the C# edition: `"1.0"`, `"2.0"`, `"3.0"`, `"4.0"`, `"5.0"`, `"6.0"`, `"7.0"`, `"8.0"`, `"9.0"`, `"10.0"`, `"11.0"`, `"12.0"` (default: `"12.0"`).
- `tokenize_csharp(source, version)` — convenience function that tokenizes C# source and returns `Vec<Token>`.
- `grammar_root()` helper that uses `PathBuf` navigation from `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Returns `Err(String)` for unrecognised version strings instead of panicking on a missing file.
- Test suite (17 tests) covering class declarations, keywords, arithmetic operators, multi-character operators, string literals, numbers, delimiters, whitespace skipping, the factory function, method declarations, versioned grammar selection for all 12 C# versions, and error cases for unknown and empty versions. Also includes tests for namespace declarations and using directives — C#-specific constructs not present in Java.
