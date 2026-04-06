# Changelog

All notable changes to the `coding-adventures-python-lexer` crate will be documented in this file.

## [0.1.0] - 2026-04-05

### Added
- `create_python_lexer(source, version)` -- factory function that loads the versioned `python{version}.tokens` grammar and returns a configured `GrammarLexer`.
- `tokenize_python(source, version)` -- convenience function that tokenizes Python source and returns `Vec<Token>`.
- Supported versions: "2.7", "3.0", "3.6", "3.8", "3.10", "3.12" (default: "3.12").
- Loads grammar files from `code/grammars/python/python{version}.tokens` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Comprehensive test suite covering simple expressions, keywords, reserved keywords, indentation, bracket suppression, operators, strings, comments, float literals, the factory function, version selection, and soft keywords (3.10+/3.12+).
