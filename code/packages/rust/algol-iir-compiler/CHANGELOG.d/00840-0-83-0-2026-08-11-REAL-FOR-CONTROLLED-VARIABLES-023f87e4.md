## 0.83.0 — 2026-08-11 — real for controlled variables

ALGOL for-clause controlled variables may now be `real` scalars or real array
elements. Integer initial values, steps, and limits widen to `f64`; loop sign,
bound, and increment operations retain the controlled variable's numeric width.

