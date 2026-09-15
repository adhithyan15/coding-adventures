## 0.265.0 — 2026-09-12 — bounded step-loop scalar recurrences

Statically bounded `step` loops now simulate one simple local scalar assignment
across every iteration, so exact integer, finite real, and boolean recurrences
retain their final snapshot. Integer and real simulations share a 4,096-step
cap; overflow, non-finite values, dynamic bounds, control writes, and compound
bodies continue to fail closed.

