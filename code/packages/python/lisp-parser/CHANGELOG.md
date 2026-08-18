# Changelog

## 0.1.1 — 2026-08-17

### Fixed

- Eliminated runtime grammar loading: `create_lisp_parser` now imports a
  pre-compiled `_grammar` module instead of reading and parsing the
  `.grammar` file from `code/grammars/` on every call. The old code walked
  out of the installed package's own directory to a monorepo-relative path
  that a published PyPI package does not ship, so `pip install` + first use
  would raise `FileNotFoundError`. The previous `_grammar.py` in this
  package was a stray, unwired artifact never imported by `parser.py`; it
  has been regenerated fresh from `code/grammars/lisp/lisp.grammar` and is
  now actually wired in.

## 0.1.0 — 2026-03-20

### Added

- **Lisp parser** — thin wrapper around grammar-tools GrammarParser
- Loads `lisp.grammar` for grammar-driven parsing
- `create_lisp_parser()` factory and `parse_lisp()` convenience function
- 6 grammar rules: program, sexpr, atom, list, list_body, quoted
- Supports dotted pairs via DOT token in list_body
- 24 tests, 100% coverage
