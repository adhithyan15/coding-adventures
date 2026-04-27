# Changelog

All notable changes to `coding_adventures_csharp_lexer` will be documented in this file.

## [0.1.0] - 2026-04-11

### Added
- Initial release
- `CodingAdventures::CSharpLexer.tokenize(source, version: nil)` method that tokenizes C# source code
- `CodingAdventures::CSharpLexer.tokenize_csharp(source, version: nil)` alias for public API parity
- `CodingAdventures::CSharpLexer.create_csharp_lexer(source, version: nil)` factory method returning a configured lexer context
- `VALID_VERSIONS` constant listing all 12 supported C# version strings: `"1.0"`, `"2.0"`, `"3.0"`, `"4.0"`, `"5.0"`, `"6.0"`, `"7.0"`, `"8.0"`, `"9.0"`, `"10.0"`, `"11.0"`, `"12.0"`
- `resolve_tokens_path(version)` class method -- resolves to `csharp/csharp<version>.tokens` (versioned) or `csharp/csharp12.0.tokens` (default)
- Raises `ArgumentError` with a descriptive message when an unknown version is given
- Loads `.tokens` grammar file and delegates to `GrammarLexer`
- Full test suite with SimpleCov coverage >= 80%
