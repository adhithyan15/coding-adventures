# Changelog — coding-adventures-lisp-parser

## Unreleased

### Fixed

- Eliminated runtime grammar loading: `parse` now requires a pre-compiled
  `_grammar` module instead of reading and parsing the `lisp.grammar` file
  from `code/grammars/` on every call. The old code walked out of the
  installed package's own directory to a monorepo-relative path that a
  published LuaRocks package does not ship.

## 0.1.0 — 2026-03-29

Initial release.

- Grammar-driven Lisp/Scheme parser using `lisp.grammar` and `GrammarParser`.
- Handles all six grammar rules: program, sexpr, atom, list, list_body, quoted.
- Supports dotted pairs (cons cell notation), quoted forms (tick shorthand),
  nested lists of arbitrary depth, and multi-expression programs.
- Public API: `parse(source)`, `create_parser(source)`, `get_grammar()`.
- Full busted test suite covering atoms, lists, nesting, quoted forms, dotted
  pairs, multi-expression programs, real Lisp programs, and error cases.
