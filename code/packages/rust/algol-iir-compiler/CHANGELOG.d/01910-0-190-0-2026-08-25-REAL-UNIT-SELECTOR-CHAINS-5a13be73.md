## 0.190.0 — 2026-08-25 — real unit selector chains

Bounded static while analysis now recognizes left-associative finite-real
identity chains composed from multiplication and division by exact positive
one. Division remains directional, non-unit terms fail closed, and the
ten-level selector-dependency cap is unchanged.

