# Changelog

All notable changes to the C# Lexer package will be documented in this file.

## [0.1.0] - 2026-04-11

### Added
- Initial release of the C# lexer package.
- `tokenize_csharp()` function that tokenizes C# source code using the grammar-driven lexer.
- `create_csharp_lexer()` factory function for creating a `GrammarLexer` configured for C#.
- `version` parameter supporting all 12 C# versions: `"1.0"`, `"2.0"`, `"3.0"`, `"4.0"`,
  `"5.0"`, `"6.0"`, `"7.0"`, `"8.0"`, `"9.0"`, `"10.0"`, `"11.0"`, `"12.0"`.
  Default is `"12.0"` (latest).
- `_resolve_tokens_path(version)` private helper mapping version strings to paths.
- Raises `ValueError` with a clear message for unknown version strings.
- Comprehensive test suite with 80%+ coverage, covering keywords, operators,
  delimiters, C#-specific operators (`?.`, `??`, `=>`), and all version strings.
