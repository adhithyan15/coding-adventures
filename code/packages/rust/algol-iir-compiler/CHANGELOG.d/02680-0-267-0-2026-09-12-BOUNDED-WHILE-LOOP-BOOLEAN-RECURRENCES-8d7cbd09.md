## 0.267.0 — 2026-09-12 — bounded while-loop boolean recurrences

Statically bounded `while` elements now simulate one simple local boolean
assignment after each true predicate, retaining the exact final snapshot under
the existing locality, dependency, termination, and 4,096-evaluation guards.

