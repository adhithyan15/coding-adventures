# Changelog — coding-adventures-lisp-parser

## 0.1.0 — 2026-03-29

Initial release.

- Grammar-driven Lisp/Scheme parser using `lisp.grammar` and `GrammarParser`.
- Handles all six grammar rules: program, sexpr, atom, list, list_body, quoted.
- Supports dotted pairs (cons cell notation), quoted forms (tick shorthand),
  nested lists of arbitrary depth, and multi-expression programs.
- Public API: `parse(source)`, `create_parser(source)`, `get_grammar()`.
- Full busted test suite covering atoms, lists, nesting, quoted forms, dotted
  pairs, multi-expression programs, real Lisp programs, and error cases.
