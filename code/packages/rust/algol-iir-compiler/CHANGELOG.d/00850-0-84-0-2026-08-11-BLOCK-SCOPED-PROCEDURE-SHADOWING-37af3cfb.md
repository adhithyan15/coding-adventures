## 0.84.0 — 2026-08-11 — block-scoped procedure shadowing

Procedure declarations now receive stable sibling-function identities while a
nearest-binding map follows ALGOL block scope. A nested declaration may shadow
an outer procedure, calls inside the block resolve to the nested sibling, and
leaving the block restores the outer binding. Same-block duplicates remain an
error, and calls retain the existing fully typed direct IIR ABI.

