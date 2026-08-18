# Changelog

## [0.1.1] - 2026-08-17

### Fixed

- Eliminated runtime grammar loading: `CssParser.create_parser/0` now imports a pre-compiled grammar module (`CodingAdventures.CssParser.Grammar`) instead of `File.read!`-ing `css.grammar` from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published Hex package does not ship, so `mix deps.get` + first use would raise `File.Error` (enoent).

## [0.1.0] - Unreleased

### Added

- Added the Elixir CSS parser wrapper over the shared grammar-driven parser.
- Added tests for empty stylesheets, qualified rules, selector lists, at-rules, parser creation, lexer error propagation, and malformed parse errors.
