# Changelog

All notable changes to `coding_adventures_csharp_parser` will be documented in this file.

## [0.1.0] - 2026-04-11

### Added
- Initial release
- `CodingAdventures::CSharpParser.parse(source, version: nil)` method that parses C# source into an AST
- `CodingAdventures::CSharpParser.parse_csharp(source, version: nil)` alias for public API parity
- `CodingAdventures::CSharpParser.create_csharp_parser(source, version: nil)` factory method returning a configured parser context
- `VALID_VERSIONS` constant listing all 12 supported C# version strings: `"1.0"`, `"2.0"`, `"3.0"`, `"4.0"`, `"5.0"`, `"6.0"`, `"7.0"`, `"8.0"`, `"9.0"`, `"10.0"`, `"11.0"`, `"12.0"`
- `resolve_grammar_path(version)` class method -- resolves to `csharp/csharp<version>.grammar` (versioned) or `csharp/csharp12.0.grammar` (default)
- Raises `ArgumentError` with a descriptive message when an unknown version is given
- Two-step pipeline: lexer tokenizes with `.tokens` file, parser builds AST with `.grammar` file
- Depends on `coding_adventures_csharp_lexer` for the tokenization step
- Full test suite with SimpleCov coverage >= 80%
