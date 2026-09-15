## 0.163.0 — 2026-08-13 — checked static idempotent assignments

Capped `while` body-effect analysis now recognizes a self-only static numeric
or boolean expression as idempotent when its checked value exactly matches the
tracked scalar. Expressions with any other dependency, unknown values,
non-finite reals, strings, arrays, globals, and by-name targets fail closed.

