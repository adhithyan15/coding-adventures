# Changelog

All notable changes to this package will be documented in this file.

## [0.1.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `tokenize`/`create_lexer` now load the pre-compiled `_grammar.rb` (via `GrammarTools::CompiledLoader`) instead of reading and parsing `lattice.tokens` from `code/grammars/` on every call. The old code walked out of the installed gem's own directory to a monorepo-relative path that a published gem does not ship, so a real `gem install` + first use would raise `Errno::ENOENT`.

## [0.1.0] - 2026-03-23

### Added

- `CodingAdventures::LatticeLexer.tokenize(source)` — tokenizes a Lattice
  source string using the `lattice.tokens` grammar file, returning an array
  of `CodingAdventures::Lexer::Token` objects.
- `CodingAdventures::LatticeLexer.create_lexer(source)` — returns a
  `GrammarLexer` instance for streaming/incremental tokenization.
- Token support for all CSS tokens plus five Lattice-specific extensions:
  `VARIABLE` (`$color`), `EQUALS_EQUALS` (`==`), `NOT_EQUALS` (`!=`),
  `GREATER_EQUALS` (`>=`), `LESS_EQUALS` (`<=`).
- `escapes: none` grammar directive ensures CSS escape sequences are
  preserved as raw text (semantic decoding is deferred to later passes).
- Grammar path resolved via 6-level relative path from `__dir__`, consistent
  with the Python reference and the json_lexer pattern.
