# Changelog

All notable changes to this package will be documented in this file.

## [0.1.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading. Previously `get_grammar/1` read
  `haskell<version>.tokens` from `code/grammars/haskell/` via `File.read!`
  at an absolute path that walks outside this package's own directory —
  this works in the monorepo but would raise a `File.Error` on first use
  after a published Hex package is installed, since `code/grammars/` is not
  part of the package. All 7 supported versions (`1.0`, `1.1`, `1.2`, `1.3`,
  `1.4`, `98`, `2010`) are now compiled ahead of time into
  `CodingAdventures.HaskellLexer.Grammar.V*` submodules (via
  `grammar-tools compile-tokens`) and looked up through a
  `version => &Grammar.V*.token_grammar/0` map, mirroring the pattern
  already used by `verilog_lexer`. `:persistent_term` caching is preserved.
  No versions required `--force`; all 7 `.tokens` files validated cleanly.
  Public API (`tokenize/2`, `create_lexer/1`, `default_version/0`,
  `supported_versions/0`, and the `ArgumentError` message for unknown
  versions) is unchanged.

## [0.1.0] - 2026-04-11

### Added
- Initial release
- `tokenize(source, version \\ nil)` now loads the requested `haskell<version>.tokens`
  grammar, caches it in `:persistent_term`, and returns `{:ok, tokens}`.
- `create_lexer(version \\ nil)` now returns the parsed `TokenGrammar` for the
  requested Haskell version.
- Added `default_version/0` and `supported_versions/0` helpers for version-aware callers.
- Version validation raises `ArgumentError` with a descriptive message for unknown versions.
- Full test suite covering all supported Haskell version strings, nil / empty version,
  grammar loading, and error cases.
