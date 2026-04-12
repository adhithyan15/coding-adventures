# Changelog — CodingAdventures::JavaLexer (Perl)

All notable changes to this package are documented here.

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
