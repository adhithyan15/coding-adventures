# Changelog

All notable changes to the C# Parser package will be documented in this file.

## [0.1.0] - 2026-04-11

### Added
- Initial release of the C# parser package.
- `parse_csharp()` function that parses C# source code into generic `ASTNode` trees.
- `create_csharp_parser()` factory function for creating a `GrammarParser` configured for C#.
- `version` parameter supporting all 12 C# versions: `"1.0"`, `"2.0"`, `"3.0"`, `"4.0"`,
  `"5.0"`, `"6.0"`, `"7.0"`, `"8.0"`, `"9.0"`, `"10.0"`, `"11.0"`, `"12.0"`.
  Default is `"12.0"` (latest).
- `_resolve_grammar_path(version)` private helper mapping version strings to grammar paths.
- Raises `ValueError` with a clear message for unknown version strings.
- Comprehensive test suite with 80%+ coverage covering class declarations, namespace
  declarations, method declarations, the factory function, and all 12 version strings.
- Depends on `coding-adventures-csharp-lexer` for tokenization.
