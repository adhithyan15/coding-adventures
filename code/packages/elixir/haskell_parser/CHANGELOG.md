# Changelog

All notable changes to this package will be documented in this file.

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
