## 0.155.0 — 2026-08-13 — stable conditional while effects

Capped `while` dependency analysis now selects a body branch controlled by a
statically known bare boolean local when an all-path scan proves the body never
writes that selector. Dynamic, written, nonlocal, and compound selectors remain
conservative.

