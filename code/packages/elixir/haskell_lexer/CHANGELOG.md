# Changelog

All notable changes to this package will be documented in this file.

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
