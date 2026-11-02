# Changelog

## [Unreleased]

### Changed

- `_grammar` regenerated from `code/grammars/ruby/ruby.grammar` (#18778),
  which gained a `for_statement` (`for x in xs [do] … end`), `rescue => e`
  with no class, a guard so a parenthesised call followed by a binary operator
  (`f() + f`) parses as one expression statement, and a lookahead that keeps a
  `[` re-tagged `[ line start` (only the Rust `ruby-lexer` produces it)
  from being an index or a paren-less call's first argument.  This package's
  lexer never produces that value, so the last change does not affect it here.
    (Generated with a byte-for-byte port of the Elixir `compile_parser_grammar`
    emitter, checked by regenerating the previous file unchanged from the
    previous grammar; no Elixir toolchain was available.)

## [0.1.1] - 2026-08-17

### Fixed

- Eliminated runtime grammar loading: `create_parser/0` now imports a pre-compiled grammar module (`CodingAdventures.RubyParser.Grammar`, generated via `grammar-tools compile-grammar`) instead of `File.read!`-ing the `.grammar` file from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published Hex package does not ship, so `mix deps.get` + first use would raise `File.Error` (enoent).
- Added regression tests covering `def`, `class`, `if`, `while`, `case`, and `begin`/`rescue` statement parsing. A stale, previously-generated compiled grammar was found in the Ruby- and TypeScript-language ports of this same package, silently missing these statement rules after `ruby.grammar` was extended past when that compiled copy was generated. This package's grammar was compiled fresh from the current `code/grammars/ruby/ruby.grammar` (never hand-written or reused), and all six rules were confirmed present and parse correctly; the new tests guard against future regeneration silently dropping them again.

## [0.1.0] - Unreleased

### Added

- Added the Elixir Ruby parser wrapper over the shared grammar-driven parser.
- Added tests for assignments, arithmetic precedence, method calls, parser creation, lexer error propagation, and malformed parse errors.
