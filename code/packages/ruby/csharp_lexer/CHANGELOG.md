# Changelog

All notable changes to `coding_adventures_csharp_lexer` will be documented in this file.

## [0.1.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `tokenize` now loads a pre-compiled `_grammar[_<version>].rb` per C# version (via `GrammarTools::CompiledLoader`, newly generated for all 12 supported versions from `csharp/csharp<version>.tokens`) instead of reading and parsing the `.tokens` file from `code/grammars/` on every call, mirroring the pattern already used by `java_lexer`. The old code walked out of the installed gem's own directory to a monorepo-relative path that a published gem does not ship, so a real `gem install` + first use would raise `Errno::ENOENT`.

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
