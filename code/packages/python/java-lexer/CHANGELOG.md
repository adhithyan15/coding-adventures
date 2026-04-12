# Changelog

All notable changes to the Java Lexer package will be documented in this file.

## [0.1.0] - 2026-04-11

### Added
- Initial release of the Java lexer package.
- `tokenize_java()` function that tokenizes Java source code using the grammar-driven lexer.
- `create_java_lexer()` factory function for creating a `GrammarLexer` configured for Java.
- `version` parameter supporting Java versions: `"1.0"`, `"1.1"`, `"1.4"`, `"5"`, `"7"`,
  `"8"`, `"10"`, `"14"`, `"17"`, `"21"`. Default is `"21"` (latest).
- `_resolve_tokens_path(version)` private helper mapping version strings to paths.
- Raises `ValueError` with a clear message for unknown version strings.
- Comprehensive test suite with 80%+ coverage.
