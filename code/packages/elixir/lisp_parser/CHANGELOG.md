# Changelog

## [0.1.1] - Unreleased

### Fixed

- Eliminated runtime grammar loading: `create_parser/0` now imports a pre-compiled grammar module (`CodingAdventures.LispParser.Grammar`) instead of `File.read!`-ing `lisp.grammar` from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published Hex package does not ship, so `mix deps.get` + first use would raise `File.Error` (enoent).

## [0.1.0] - Unreleased

### Added

- Added the Elixir Lisp parser wrapper over the shared grammar-driven parser.
- Added tests for lists, quoted forms, dotted pairs, parser creation, invalid input propagation, and malformed list errors.
