# Changelog — mccarthy-lisp-parser

## v0.1.0 — 2026-06-03 — initial release (L1)

McCarthy 1960 Lisp parser.

* `LispExpr` AST: `Nil`, `Symbol(String)`, `Int(i64)`,
  `Cons(Box<LispExpr>, Box<LispExpr>)`.
* `parse(&str)` — re-tokenize + parse.
* `parse_tokens(&[TokenWithLoc])` — parse a pre-tokenized stream.
* Sugar: `'X` → `(QUOTE X)`; standard list nesting via NIL
  terminator; dotted-pair literals parse to `Cons` directly.
* Helpers: `LispExpr::list([a, b, c])` builds the nested-Cons
  encoding, `LispExpr::quote(inner)` wraps in QUOTE.

`ParseError` is structured (`UnexpectedToken`, `UnexpectedEof`,
`StrayDot`, `MultipleDotsInList`, etc.) so downstream
compiler diagnostics can point at the right location.

Tests pin the 1960-paper examples: `(CAR '(A B C))`,
`(LAMBDA (X) X)`, `(LABEL FF (LAMBDA …))`, dotted-pair
literals, and every error path.
