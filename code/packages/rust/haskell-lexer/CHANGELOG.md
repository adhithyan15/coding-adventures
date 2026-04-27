# Changelog

All notable changes to the `coding-adventures-haskell-lexer` crate will be documented in this file.

## [0.1.0] - 2026-04-11

### Added
- `create_haskell_lexer(source, version)` — factory function that loads the appropriate `haskell{version}.tokens` grammar and returns a configured `GrammarLexer`. The `version` parameter selects the Haskell edition: `"1.0"`, `"1.1"`, `"1.4"`, `"5"`, `"7"`, `"8"`, `"10"`, `"14"`, `"17"`, `"21"` (default: `"21"`).
- `tokenize_haskell(source, version)` — convenience function that tokenizes Haskell source and returns `Vec<Token>`.
- `grammar_root()` helper that uses `PathBuf` navigation from `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Returns `Err(String)` for unrecognised version strings instead of panicking on a missing file.
- Test suite covering class declarations, keywords, operators, string literals, numbers, delimiters, whitespace, factory function, versioned grammar selection, and error cases for unknown versions.
