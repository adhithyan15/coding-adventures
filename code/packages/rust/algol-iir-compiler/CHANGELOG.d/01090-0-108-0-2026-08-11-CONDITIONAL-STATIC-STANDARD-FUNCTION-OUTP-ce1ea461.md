## 0.108.0 — 2026-08-11 — conditional static standard-function output

Runtime conditionals may now select between formatter-free real branches that
contain composed `abs` and exact-`sqrt` calls. Both branches are validated with
lexical override, domain, exact-root, and finiteness checks before the compiler
emits the existing portable string-output control flow.

