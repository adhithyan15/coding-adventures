## 0.77.0 — 2026-08-11 — block-scoped switch shadowing

Switch declarations now follow lexical block scope. A nested block may shadow
an outer switch, nearest-binding lookup applies while the block is active, and
the outer binding is restored on exit. Duplicate switches in one block remain
an error, and existing deferred designator and cycle checks are unchanged.

