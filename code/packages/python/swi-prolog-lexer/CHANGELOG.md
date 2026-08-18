# Changelog

## [0.1.1] - 2026-08-17

### Fixed

- Eliminated runtime grammar loading: `create_swi_prolog_lexer` now
  imports a pre-compiled `_grammar` module instead of reading and parsing
  the `.tokens` file from `code/grammars/` on every call. The old code
  walked out of the installed package's own directory to a
  monorepo-relative path that a published PyPI package does not ship, so
  `pip install` + first use would raise `FileNotFoundError`.

## Unreleased

- Tokenize SWI CLP(FD) range syntax (`..`) as one symbolic atom so
  finite-domain expressions such as `1..4` can parse as operator terms.

## 0.1.0

- Added the first SWI-Prolog lexer package backed by `code/grammars/prolog/swi.tokens`
