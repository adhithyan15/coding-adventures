# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-11

### Added
- Initial release
- `parse_csharp(source, version \\ nil)` -- parses C# source code with optional
  version parameter accepting `"1.0"`, `"2.0"`, `"3.0"`, `"4.0"`, `"5.0"`, `"6.0"`,
  `"7.0"`, `"8.0"`, `"9.0"`, `"10.0"`, `"11.0"`, `"12.0"`.
  Passing `nil` (default) uses C# 12.0 grammar. Returns a map AST node with
  `:rule_name`, `:children`, and `:version` keys.
- `create_csharp_parser(source, version \\ nil)` -- factory function returning a parser
  context map for use in pipeline-style parsing workflows.
- Version validation raises `ArgumentError` with a descriptive message for unknown versions.
- Full test suite covering all 12 supported C# version strings, nil version, factory
  function, and error cases.
- Depends on `csharp_lexer` as a path dependency for token production.
