# prolog-operator-parser

`prolog-operator-parser` is the first parser layer that actually consults a
`prolog-core` operator table while parsing terms, goals, clauses, and queries.

It operates on token streams produced by dialect lexers. Dialect packages keep
owning their lexers and grammar files; this package owns the shared
operator-aware parsing path.

It now also expands DCG rules (`-->`) into ordinary executable clauses while
parsing, including list terminals and braced `{Goal}` escapes.
