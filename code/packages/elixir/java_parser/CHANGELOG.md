# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-11

### Added
- Initial release
- `parse(source, version \\ nil)` -- parses Java source code into ASTs with optional version parameter
  accepting `"1.0"`, `"1.1"`, `"1.4"`, `"5"`, `"7"`, `"8"`, `"10"`, `"14"`, `"17"`, `"21"`.
  Passing `nil` (default) uses Java 21 grammar.
- `create_parser(source, version \\ nil)` -- factory function returning a parser context map
  for use in pipeline-style parsing workflows.
- Version validation raises `ArgumentError` with a descriptive message for unknown versions.
- Full test suite covering all supported Java version strings, nil version, factory
  function, and error cases.
