# Changelog — CodingAdventures::PythonLexer (Perl)

All notable changes to this package are documented here.

## [Unreleased] — 2026-08-17

### Fixed

- Eliminated runtime grammar-file disk reads. Previously, `_grammar($version)`
  opened `code/grammars/python/python<version>.tokens` (or, for `'legacy'`,
  the unversioned `code/grammars/python/python.tokens`) off disk at
  runtime, using a path that walked outside this package's own directory
  into the monorepo (`File::Basename`/`File::Spec`-based directory
  walk-up). That works inside a checkout of the monorepo, but a published
  CPAN distribution does not include `code/grammars/` — installing this
  package and calling `tokenize` would die with "cannot open ... No such
  file or directory".
- All 7 supported grammars (the 6 versions in `@SUPPORTED_VERSIONS` —
  `2.7`, `3.0`, `3.6`, `3.8`, `3.10`, `3.12` — plus the unversioned
  `'legacy'` grammar) are now compiled ahead of time (via
  `grammar-tools compile-tokens`) into `_Grammar_<version>.pm` sibling
  modules under `lib/CodingAdventures/PythonLexer/` (e.g.
  `_Grammar_3_12.pm`, `_Grammar_legacy.pm`) that embed the parsed
  `TokenGrammar` as native Perl data. `_grammar($version)` now `require`s
  the appropriate precompiled module and calls its `token_grammar()`
  constructor (cached per version) instead of reading and parsing a
  `.tokens` file from disk.
- Removed `_grammar_path($version)` and `_grammars_dir()` (now dead)
  along with the `File::Basename`/`File::Spec` prerequisites in
  `Makefile.PL`.
- `_grammar($version)` now dies with a descriptive "unknown Python
  version" message for versions with no compiled module, instead of the
  prior "cannot open" disk-read error.
- Public API (`tokenize`), `DEFAULT_VERSION`, and `@SUPPORTED_VERSIONS`
  are unchanged.

## [0.01] — 2026-03-29

### Added

- Initial implementation of `CodingAdventures::PythonLexer`.
- `tokenize($source)` — class method that tokenizes a Python string using
  the shared `python.tokens` grammar compiled to Perl `qr//` patterns.
- Grammar loaded from `code/grammars/python.tokens` and cached for the
  process lifetime using package-level variables.
- Path navigation uses `__FILE__` and `File::Basename::dirname` to locate
  the grammar file without hardcoded paths (climbs 5 levels from the module
  file to reach `code/`, then descends into `grammars/`).
- Full token set: NAME, NUMBER, STRING; keyword tokens (IF, ELIF, ELSE,
  WHILE, FOR, DEF, RETURN, CLASS, IMPORT, FROM, AS, TRUE, FALSE, NONE);
  operator tokens (EQUALS_EQUALS, EQUALS, PLUS, MINUS, STAR, SLASH);
  delimiter tokens (LPAREN, RPAREN, COMMA, COLON).
- Line and column tracking throughout tokenization.
- `die` with descriptive message on unexpected input characters.
- EOF sentinel always appended as last token.
- Comprehensive `Test2::V0` test suite covering keywords, identifiers,
  numbers, strings, operators, punctuation, composite expressions,
  whitespace handling, position tracking, and error cases.
- `BUILD` script with transitive dependency installation in leaf-to-root
  order via cpanm.
- `BUILD_windows` stub (Perl testing not supported on Windows).
