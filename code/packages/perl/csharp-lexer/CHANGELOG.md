# Changelog — CodingAdventures::CSharpLexer (Perl)

All notable changes to this package are documented here.

## [Unreleased] — 2026-08-17

### Fixed

- Eliminated runtime grammar-file disk reads. Previously, `_grammar($version)`
  opened `code/grammars/csharp/csharp<version>.tokens` off disk at runtime,
  using a path that walked outside this package's own directory into the
  monorepo (`File::Basename`/`File::Spec`-based directory walk-up). That
  works inside a checkout of the monorepo, but a published CPAN
  distribution does not include `code/grammars/` — installing this package
  and calling `tokenize` would die with "cannot open ... No such file or
  directory".
- Each of the 12 supported C# versions' `.tokens` grammar is now compiled
  ahead of time (via `grammar-tools compile-tokens`) into a
  `_Grammar_<version>.pm` sibling module under
  `lib/CodingAdventures/CSharpLexer/` (e.g. `_Grammar_12_0.pm`) that embeds
  the parsed `TokenGrammar` as native Perl data. `_grammar($version)` now
  `require`s the appropriate precompiled module and calls its
  `token_grammar()` constructor (cached per version) instead of reading
  and parsing a `.tokens` file from disk.
- Removed `_resolve_tokens_path($version)` and `_grammars_dir()` (now
  dead) along with the `File::Basename`/`File::Spec` prerequisites in
  `Makefile.PL`.
- Public API (`tokenize`, `tokenize_csharp`, `new_csharp_lexer`), version
  validation, and error message text are unchanged.

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
