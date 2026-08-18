# Changelog — CodingAdventures::JavaLexer (Perl)

All notable changes to this package are documented here.

## [Unreleased] — 2026-08-17

### Fixed

- Eliminated runtime grammar-file disk reads. Previously, `_grammar($version)`
  opened `code/grammars/java/java<version>.tokens` off disk at runtime,
  using a path that walked outside this package's own directory into the
  monorepo (`File::Basename`/`File::Spec`-based directory walk-up). That
  works inside a checkout of the monorepo, but a published CPAN
  distribution does not include `code/grammars/` — installing this package
  and calling `tokenize` would die with "cannot open ... No such file or
  directory".
- Each of the 10 supported Java versions' `.tokens` grammar is now compiled
  ahead of time (via `grammar-tools compile-tokens`) into a
  `_Grammar_<version>.pm` sibling module under
  `lib/CodingAdventures/JavaLexer/` (e.g. `_Grammar_1_4.pm`, `_Grammar_21.pm`)
  that embeds the parsed `TokenGrammar` as native Perl data. `_grammar($version)`
  now `require`s the appropriate precompiled module and calls its
  `token_grammar()` constructor (cached per version) instead of reading
  and parsing a `.tokens` file from disk.
- Removed `_resolve_tokens_path($version)` and `_grammars_dir()` (now
  dead) along with the `File::Basename`/`File::Spec` prerequisites in
  `Makefile.PL`.
- Public API (`tokenize`), version validation, and error message text are
  unchanged.

## [0.01] — 2026-04-11

### Added

- Initial implementation of `CodingAdventures::JavaLexer`.
- `tokenize($source, $version)` — tokenizes a Java string using rules compiled
  from the shared `java/java<version>.tokens` grammar file.
- Optional `$version` parameter selects a versioned grammar file under
  `code/grammars/java/`.
- Valid version strings: `"1.0"`, `"1.1"`, `"1.4"`, `"5"`, `"7"`, `"8"`,
  `"10"`, `"14"`, `"17"`, `"21"`. Default is `"21"`.
- `_resolve_tokens_path($version)` — internal helper that maps version
  strings to grammar file paths.
- Per-version caches for grammar, compiled rules, skip rules, and keyword
  map (hashes keyed by version string).
- Validation: unknown version strings raise a descriptive `die` immediately.
- Security: rejects regex patterns containing Perl code-execution constructs
  (`(?{ ... })` and `(??{ ... })`).
- `t/00-load.t` — smoke test that the module loads and has a VERSION.
- `t/01-basic.t` — comprehensive test suite covering keywords, identifiers,
  numbers, strings, operators, punctuation, composite expressions, whitespace
  handling, position tracking, version-aware tokenization, cache consistency,
  and error cases.
- `BUILD` and `BUILD_windows` scripts.
- `Makefile.PL`, `cpanfile`, `README.md`, `CHANGELOG.md`.
