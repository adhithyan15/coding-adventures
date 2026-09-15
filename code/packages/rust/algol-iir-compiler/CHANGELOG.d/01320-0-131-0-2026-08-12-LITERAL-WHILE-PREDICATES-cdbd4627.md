## 0.131.0 — 2026-08-12 — literal while predicates

The initial `while` proof now recognizes direct boolean literals. A `true`
predicate establishes the first body execution, while `false` remains a
zero-trip path that cannot establish definite string initialization.

