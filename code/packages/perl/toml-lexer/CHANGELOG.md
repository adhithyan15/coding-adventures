# Changelog — CodingAdventures::TomlLexer (Perl)

All notable changes to this package will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This package uses [Semantic Versioning](https://semver.org/).

## [Unreleased] - 2026-08-17

### Changed

- **Eliminated runtime grammar-file disk reads.** `_grammar()` previously
  opened `code/grammars/toml/toml.tokens` off disk (via `_grammars_dir()`
  climbing 5 directory levels from `__FILE__` with `File::Basename::dirname`
  and `File::Spec`) and parsed it with
  `CodingAdventures::GrammarTools::parse_token_grammar` on first use. A real
  CPAN install of this package would not ship `code/grammars/`, so that path
  would die with "cannot open ... No such file or directory" outside this
  monorepo checkout.
- `toml.tokens` is now compiled once, at dev time, via
  `code/programs/perl/grammar-tools/grammar-tools.pl compile-tokens` into a
  checked-in generated module, `CodingAdventures::TomlLexer::_Grammar`
  (`lib/CodingAdventures/TomlLexer/_Grammar.pm`), which defines
  `token_grammar()`. `_grammar()` now `require`s that module and calls the
  qualified sub instead of touching disk.
- Removed the now-dead `_grammars_dir()` sub and the `use File::Basename
  qw(dirname);` / `use File::Spec;` imports (no longer used anywhere else
  in the module).
- Removed `File::Basename` and `File::Spec` from `Makefile.PL`'s
  `PREREQ_PM`.
- No behavioral change: `_build_rules()`, `tokenize()`, and the
  Perl-code-execution regex security check remain untouched.

## [0.01] - 2026-03-29

### Added

- Initial implementation of the TOML lexer Perl package.
- `tokenize($source)` class method — tokenizes a TOML source string using
  the grammar-driven infrastructure, returning an arrayref of token hashrefs.
  Each hashref has keys: `type`, `value`, `line`, `col`.
- Grammar loading with caching — the `toml.tokens` file is read and parsed
  once per process; subsequent calls reuse the cached `TokenGrammar`.
- Path navigation — locates `toml.tokens` by climbing 5 directory levels
  from `lib/CodingAdventures/` to the `code/` repo root, then descending
  into `grammars/`.
- Pattern compilation — both regex and literal token definitions are compiled
  into `\G`-anchored `qr//` patterns for efficient position-based matching.
- Skip-first algorithm — skip patterns (horizontal whitespace, TOML comments)
  are tried at each position before token patterns.
- Full test suite covering:
  - Module loading and VERSION check (t/00-load.t)
  - Empty and whitespace-only inputs (t/01-basic.t)
  - Key-value pairs (BARE_KEY, EQUALS, BASIC_STRING)
  - Table headers `[section]` and `[[array-of-tables]]`
  - All four TOML string types (basic, literal, multi-line basic, multi-line literal)
  - All integer forms (decimal, hex, octal, binary, underscore-separated)
  - All float forms (decimal, scientific, inf, -inf, nan)
  - Boolean literals (true, false)
  - All date/time types (OFFSET_DATETIME, LOCAL_DATETIME, LOCAL_DATE, LOCAL_TIME)
  - Inline tables `{ key = "val" }`
  - Arrays `[1, 2, 3]`
  - Whitespace and comment consumption
  - Token position tracking (line, col)
  - Error on unexpected character
- `Makefile.PL` with correct PREREQ_PM entries.
- `cpanfile` listing runtime and test dependencies.
- `BUILD` script installing all transitive deps leaf-to-root via cpanm.
- `BUILD_windows` skipping Perl tests (not supported on Windows CI).
