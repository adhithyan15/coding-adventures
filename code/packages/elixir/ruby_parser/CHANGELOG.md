# Changelog

## [0.1.1] - 2026-08-17

### Fixed

- Eliminated runtime grammar loading: `create_parser/0` now imports a pre-compiled grammar module (`CodingAdventures.RubyParser.Grammar`, generated via `grammar-tools compile-grammar`) instead of `File.read!`-ing the `.grammar` file from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published Hex package does not ship, so `mix deps.get` + first use would raise `File.Error` (enoent).
- Added regression tests covering `def`, `class`, `if`, `while`, `case`, and `begin`/`rescue` statement parsing. A stale, previously-generated compiled grammar was found in the Ruby- and TypeScript-language ports of this same package, silently missing these statement rules after `ruby.grammar` was extended past when that compiled copy was generated. This package's grammar was compiled fresh from the current `code/grammars/ruby/ruby.grammar` (never hand-written or reused), and all six rules were confirmed present and parse correctly; the new tests guard against future regeneration silently dropping them again.

## [0.1.0] - Unreleased

### Added

- Added the Elixir Ruby parser wrapper over the shared grammar-driven parser.
- Added tests for assignments, arithmetic precedence, method calls, parser creation, lexer error propagation, and malformed parse errors.
