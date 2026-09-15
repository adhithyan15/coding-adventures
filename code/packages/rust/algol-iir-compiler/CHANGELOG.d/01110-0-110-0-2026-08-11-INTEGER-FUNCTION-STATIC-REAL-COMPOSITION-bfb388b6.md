## 0.110.0 — 2026-08-11 — integer-function static real composition

Exact `sign` and safely bounded `entier` results may now widen into the
formatter-free literal-only real arithmetic evaluator. `entier` is restricted
to binary64's exact integer range; larger results continue to fail closed
instead of silently losing integer precision.

