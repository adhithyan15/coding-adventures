## 0.166.0 — 2026-08-13 — conditional transitive idempotence

Transitive assignment dependencies now remain stable when every leaf of a
conditional expression is the same bare scalar, even with a dynamic selector.
Differing leaves and computed or cyclic dependency effects still fail closed.

