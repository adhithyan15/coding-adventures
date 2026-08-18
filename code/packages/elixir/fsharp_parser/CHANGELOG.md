# Changelog

All notable changes to this package will be documented in this file.

## [0.1.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `create_parser/1` now fetches the
  pre-compiled `CodingAdventures.FSharpParser.Grammar.V<version>` module
  for all 15 supported versions instead of `File.read!`-ing
  `fsharp<version>.grammar` from `code/grammars/` on every call. The old
  code walked out of the installed package's own directory to a
  monorepo-relative path that a published Hex package does not ship, so
  `mix deps.get` + first use would raise a `File.Error`. This required
  fixing a gap in the shared `grammar_tools` compiler:
  `list_expression`'s `LBRACKET ! LESS_THAN ...` negative lookahead — byte-
  identical across all 15 `fsharp<version>.grammar` files — previously
  crashed `compile-grammar` (now handled, see `grammar_tools`'s own
  CHANGELOG). No `--force` was needed once that was fixed.

## [0.1.0] - 2026-04-11

### Added
- Initial release
- `parse(source, version \\ nil)` now tokenizes with
  `CodingAdventures.FSharpLexer`, loads the requested `fsharp<version>.grammar`,
  and returns `{:ok, ast}` or `{:error, reason}`.
- `create_parser(version \\ nil)` now returns the parsed `ParserGrammar` for the
  requested F# version.
- Added `default_version/0` and `supported_versions/0` helpers for
  version-aware callers.
- Version validation raises `ArgumentError` with a descriptive message for
  unknown versions.
- Full test suite covering all supported F# version strings, nil / empty
  version, grammar loading, AST shape, and error cases.
