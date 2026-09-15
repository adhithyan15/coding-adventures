## 0.157.0 — 2026-08-13 — stable scalar while predicates

Capped `while` dependency analysis now selects a body branch controlled by a
statically known predicate over local boolean, integer, and real scalars when
an all-path scan proves the body never writes any dependency. Written, global,
array, by-name, controlled, string, and otherwise unknown dependencies remain
conservative.

