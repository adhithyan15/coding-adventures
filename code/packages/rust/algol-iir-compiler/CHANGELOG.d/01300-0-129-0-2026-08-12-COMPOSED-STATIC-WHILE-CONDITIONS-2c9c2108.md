## 0.129.0 — 2026-08-12 — composed static while conditions

The initial `while` condition proof now composes already-known finite numeric
comparisons through `not`, `and`, `or`, `impl`, and `eqv`. Every required leaf
must remain statically known; bare boolean literals and dynamic operands still
fail closed.

