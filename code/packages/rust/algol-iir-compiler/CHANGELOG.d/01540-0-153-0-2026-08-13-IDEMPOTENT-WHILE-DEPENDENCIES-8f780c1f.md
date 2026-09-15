## 0.153.0 — 2026-08-13 — idempotent while dependencies

Capped `while` control analysis now permits an exact scalar self-assignment to
a stable local dependency in the loop body. Computed assignments, array targets,
nested controls, globals, and by-name values remain conservative.

