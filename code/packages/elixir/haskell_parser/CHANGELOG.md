# Changelog

All notable changes to this package will be documented in this file.

## [0.1.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading. Previously `get_grammar/1` read
  `haskell<version>.grammar` from `code/grammars/haskell/` via `File.read!`
  at an absolute path that walks outside this package's own directory —
  this works in the monorepo but would raise a `File.Error` on first use
  after a published Hex package is installed, since `code/grammars/` is not
  part of the package. All 7 supported versions (`1.0`, `1.1`, `1.2`, `1.3`,
  `1.4`, `98`, `2010`) are now compiled ahead of time into
  `CodingAdventures.HaskellParser.Grammar.V*` submodules (via
  `grammar-tools compile-grammar`) and looked up through a
  `version => &Grammar.V*.parser_grammar/0` map, mirroring the pattern
  already used by `verilog_lexer`. `:persistent_term` caching is preserved.
  No versions required `--force`; all 7 `.grammar` files validated cleanly.
  Public API (`parse/2`, `create_parser/1`, `default_version/0`,
  `supported_versions/0`, and the `ArgumentError` message for unknown
  versions) is unchanged.

## [0.1.0] - 2026-04-11

### Added
- Initial release
- `parse(source, version \\ nil)` now tokenizes with `CodingAdventures.HaskellLexer`,
  loads the requested `haskell<version>.grammar`, and returns `{:ok, ast}` or `{:error, reason}`.
- `create_parser(version \\ nil)` now returns the parsed `ParserGrammar` for the
  requested Haskell version.
- Added `default_version/0` and `supported_versions/0` helpers for version-aware callers.
- Version validation raises `ArgumentError` with a descriptive message for unknown versions.
- Full test suite covering all supported Haskell version strings, nil / empty version,
  grammar loading, AST shape, and error cases.
