## 0.159.0 — 2026-08-13 — idempotent while-predicate dependencies

Capped `while` body-effect analysis now treats an exact scalar self-assignment
as preserving a static body-predicate dependency. Computed assignments and
nested controlled variables remain conservative.

