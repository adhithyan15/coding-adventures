## 0.119.0 — 2026-08-11 — path-independent conditional snapshots

Integer and real assignments through runtime conditional expressions now keep
static metadata when both independently proven branches have exactly the same
value. The condition still lowers normally, while differing, dynamic, or
non-finite branches fail closed.

