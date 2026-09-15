## 0.185.0 — 2026-08-24 — boolean identity selector writes

Bounded static while analysis now recognizes `x and true`, `x or false`,
`x eqv true`, and `true impl x` (including valid symmetric forms) as exact
boolean identity writes. These computed assignments preserve transitive
selector dependencies without widening the ten-level recursion cap; boolean
non-identities and unsupported computed writes still fail closed.

