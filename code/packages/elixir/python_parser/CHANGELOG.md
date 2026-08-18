# Changelog

## [0.1.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading. Previously `create_parser/0` read the
  single shared `python.grammar` from `code/grammars/python/` via
  `File.read!` at an absolute path that walks outside this package's own
  directory — this works in the monorepo but would raise a `File.Error` on
  first use after a published Hex package is installed, since
  `code/grammars/` is not part of the package. Unlike the lexer half (which
  is versioned per Python release), the parser has always used one shared
  grammar regardless of the requested Python version — that existing
  asymmetry is preserved unchanged. `python.grammar` is now compiled ahead
  of time (via `grammar-tools compile-grammar`) into a single
  `CodingAdventures.PythonParser.Grammar.parser_grammar/0` module, mirroring
  the pattern used by `verilog_lexer` but without a per-version map, since
  there is only one grammar to select. `:persistent_term` caching is
  preserved. No `--force` was required; `python.grammar` validated cleanly.
  Public API (`parse/2`, `create_parser/0`, `default_version/0`,
  `supported_versions/0`, and the `ArgumentError` message for unknown
  versions) is unchanged.

## [0.1.0] - Unreleased

### Added

- Added the Elixir Python parser wrapper over the shared grammar-driven parser.
- Added version-aware lexing for Python 2.7, 3.0, 3.6, 3.8, 3.10, and 3.12 before parsing with the shared Python subset grammar.
- Added tests for assignments, arithmetic precedence, version selection, parser caching, lexer errors, and malformed syntax.
