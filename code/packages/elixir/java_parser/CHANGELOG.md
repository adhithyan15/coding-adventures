# Changelog

All notable changes to this package will be documented in this file.

## [0.1.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading. Previously `get_grammar/1` read
  `java<version>.grammar` from `code/grammars/java/` via `File.read!` at an
  absolute path that walks outside this package's own directory — this
  works in the monorepo but would raise a `File.Error` on first use after a
  published Hex package is installed, since `code/grammars/` is not part of
  the package. All 10 supported versions (`1.0`, `1.1`, `1.4`, `5`, `7`,
  `8`, `10`, `14`, `17`, `21`) are now compiled ahead of time into
  `CodingAdventures.JavaParser.Grammar.V*` submodules (via
  `grammar-tools compile-grammar`) and looked up through a
  `version => &Grammar.V*.parser_grammar/0` map, mirroring the pattern
  already used by `verilog_lexer`. `:persistent_term` caching is preserved.
  Public API (`parse/2`, `create_parser/1`, `default_version/0`,
  `supported_versions/0`, and the `ArgumentError` message for unknown
  versions) is unchanged.
- All 10 `java<version>.grammar` files needed `--force` to compile: each
  one's `compilation_unit` rule (the grammar's root/start rule) is flagged
  by the validator as "defined but never referenced (unreachable)". This is
  a pre-existing false positive in the reachability check — a start rule is
  never referenced by another rule by definition — that never mattered
  while the runtime path only parsed the `.grammar` file and never
  validated it. The generated code is unaffected; only the validator's
  warning is suppressed via `--force`.

## [0.1.0] - 2026-04-11

### Added
- Initial release
- `parse(source, version \\ nil)` now tokenizes with `CodingAdventures.JavaLexer`,
  loads the requested `java<version>.grammar`, and returns `{:ok, ast}` or `{:error, reason}`.
- `create_parser(version \\ nil)` now returns the parsed `ParserGrammar` for the
  requested Java version.
- Added `default_version/0` and `supported_versions/0` helpers for version-aware callers.
- Version validation raises `ArgumentError` with a descriptive message for unknown versions.
- Full test suite covering all supported Java version strings, nil / empty version,
  grammar loading, AST shape, and error cases.
