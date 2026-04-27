# Changelog

## [0.1.0] - 2026-04-11

### Added
- Initial release of the Go C# lexer package.
- `TokenizeCSharp()` function that tokenizes C# source code using the grammar-driven lexer.
- `NewCSharpLexer()` factory function.
- Version support for C# 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, and 12.0.
- Default version is C# 12.0 (latest, shipped with .NET 8 LTS) when no version is specified.
- Grammar files loaded from `code/grammars/csharp/csharp{version}.tokens`.
- `required_capabilities.json` declaring all 12 allowed grammar file paths.
- Comprehensive tests for all supported versions plus error paths.
