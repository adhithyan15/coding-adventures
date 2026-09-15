## 0.162.0 — 2026-08-13 — stable conditional assignment selectors

Capped `while` body-effect analysis may now select a conditional assignment
leaf using a statically known predicate over local boolean, integer, and real
scalars that the body does not change. Written, controlled, global, array,
by-name, string, and otherwise unknown selector dependencies fail closed.

