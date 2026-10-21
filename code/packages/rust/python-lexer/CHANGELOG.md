# Changelog

All notable changes to the `coding-adventures-python-lexer` crate will be documented in this file.

## [0.2.0] - 2026-10-20

### Added — LANG84 phase 7a release tables
- `releases::{token_grammar, grammar_class, SUPPORTED_RELEASES}`: one compiled
  `TokenGrammar` per distinct `python<release>.tokens`, selected by exact
  `code/grammars/python/releases.csv` ID (`"3.13.16"`), `None` otherwise.
  Phase 7a pairs 3.0a1, 3.11.17, 3.12.3, 3.13.16 and 3.16.0a1. They compile to
  three classes: 3.12.3, 3.13.16 and 3.16.0a1 share one.
- Tests: inventory order, class sharing, and that release IDs and minor-line
  versions never select each other's tables.

### Changed
- `_grammar.rs` is generated as a LANG84 release family
  (`grammar-tools generate-rust-compiled-grammars python`). The legacy
  minor-line tables moved to `_grammar::legacy_versions`. `tokenize_python`,
  `create_python_lexer`, `SUPPORTED_VERSIONS` and `DEFAULT_VERSION` behave
  exactly as before.

## [0.1.0] - 2026-04-05

### Added
- `create_python_lexer(source, version)` -- factory function that loads the versioned `python{version}.tokens` grammar and returns a configured `GrammarLexer`.
- `tokenize_python(source, version)` -- convenience function that tokenizes Python source and returns `Vec<Token>`.
- Supported versions: "2.7", "3.0", "3.6", "3.8", "3.10", "3.12" (default: "3.12").
- Loads grammar files from `code/grammars/python/python{version}.tokens` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Comprehensive test suite covering simple expressions, keywords, reserved keywords, indentation, bracket suppression, operators, strings, comments, float literals, the factory function, version selection, and soft keywords (3.10+/3.12+).
