## 0.113.0 — 2026-08-11 — static real scalar expressions

Tracked straight-line real locals may now participate in the same bounded,
finite arithmetic and exact standard-function evaluator used by direct static
output. Runtime or invalidated operands continue to fail closed rather than
materializing a partial formatter.

