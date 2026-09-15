## 0.229.0 — 2026-08-28 — composed tracked standard-function exponents

Real-base power unrolling now accepts checked integer arithmetic that composes
pure built-in `abs` and `sign` calls over exact local snapshots. User
overrides, conditionals, globals, invalidated values, overflow, and oversized
results remain on the dynamic `f64_pow` path.

