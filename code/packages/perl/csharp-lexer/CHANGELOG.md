# Changelog — CodingAdventures::CSharpLexer (Perl)

All notable changes to this package are documented here.

## [0.01] — 2026-04-11

### Added

- Initial implementation of `CodingAdventures::CSharpLexer`.
- `tokenize($source, $version)` — tokenizes a C# string using rules compiled
  from the shared `csharp/csharp<version>.tokens` grammar file.
- Optional `$version` parameter selects a versioned grammar file under
  `code/grammars/csharp/`.
- Valid version strings: `"1.0"`, `"2.0"`, `"3.0"`, `"4.0"`, `"5.0"`,
  `"6.0"`, `"7.0"`, `"8.0"`, `"9.0"`, `"10.0"`, `"11.0"`, `"12.0"`.
  Default is `"12.0"`.
- `tokenize_csharp($source, $version)` — standalone convenience function.
- `new_csharp_lexer($source, $version)` — convenience function synonym.
- `_resolve_tokens_path($version)` — internal helper that maps version
  strings to grammar file paths.
- Per-version caches for grammar, compiled rules, skip rules, and keyword
  map (hashes keyed by version string).
- Validation: unknown version strings raise a descriptive `die` immediately.
- Security: rejects regex patterns containing Perl code-execution constructs
  (`(?{ ... })` and `(??{ ... })`).
- `t/00-load.t` — smoke test that the module loads and has a VERSION.
- `t/01-basic.t` — comprehensive test suite covering:
  - Empty and whitespace-only input
  - Identifiers
  - Number and string tokens
  - Punctuation (parens, braces, brackets, semicolon, comma, dot)
  - Basic C# class declaration
  - C# keywords (int, string, bool, new, namespace, using)
  - C# operators including `??` (null-coalescing) and `?.` (null-conditional)
  - Whitespace handling
  - Position tracking
  - All 12 version strings
  - Grammar caching
  - Error handling (unknown version, invalid version string)
- `BUILD` and `BUILD_windows` scripts.
- `Makefile.PL`, `cpanfile`, `README.md`, `CHANGELOG.md`.
- `required_capabilities.json`.
