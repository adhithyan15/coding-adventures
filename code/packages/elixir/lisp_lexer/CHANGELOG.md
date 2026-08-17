# Changelog

## [0.1.1] - Unreleased

### Fixed

- Eliminated runtime grammar loading: `create_lexer/0` now imports a pre-compiled grammar module (`CodingAdventures.LispLexer.Grammar`) instead of `File.read!`-ing `lisp.tokens` from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published Hex package does not ship, so `mix deps.get` + first use would raise `File.Error` (enoent).

## [0.1.0] - Unreleased

### Added

- Added the Elixir Lisp lexer wrapper over the shared grammar-driven lexer.
- Added tests for basic lists, operator symbols, comments, quoted dotted pairs, grammar creation, and invalid input errors.
