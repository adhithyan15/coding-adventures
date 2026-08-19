# Changelog

## [0.1.1] - 2026-08-17

### Fixed

- Eliminated runtime grammar loading: `create_swi_prolog_parser` now
  imports a pre-compiled `_grammar` module instead of reading and parsing
  the `.grammar` file from `code/grammars/` on every call. The old code
  walked out of the installed package's own directory to a
  monorepo-relative path that a published PyPI package does not ship, so
  `pip install` + first use would raise `FileNotFoundError`.

## Unreleased

- Fixed `BUILD_windows`: it now installs `../prolog-core` and
  `../prolog-operator-parser`, declared runtime dependencies that `BUILD` already
  installed.
- Parse natural SWI CLP(FD) infix syntax such as `X in 1..4`,
  `[X,Y] ins 1..4`, and `Z #= X + Y`.
- Expose `parse_swi_term(...)` for single-term parsing with named variable
  bindings.

## 0.1.0

- Added the first SWI-Prolog parser package backed by `code/grammars/prolog/swi.grammar`
- Added top-level directive collection for `:- ... .` statements
- Added grammar and executable-source support for DCG rules (`-->`)
