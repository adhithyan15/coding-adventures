## 0.158.0 — 2026-08-13 — stable standard-function while predicates

Capped `while` body-effect analysis now permits supported deterministic
standard functions to wrap stable scalar predicate dependencies. User-shadowed
and unsupported calls remain conservative.

