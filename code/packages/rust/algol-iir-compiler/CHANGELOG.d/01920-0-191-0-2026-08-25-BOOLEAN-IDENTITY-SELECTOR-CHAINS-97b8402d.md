## 0.191.0 — 2026-08-25 — boolean identity selector chains

Bounded static while analysis now recognizes left-associative boolean identity
chains composed from `and true`, `or false`, or `eqv true`. Implication retains
its directional single-operation rule, unsupported terms fail closed, and the
ten-level selector-dependency cap is unchanged.

