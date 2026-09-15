## 0.237.0 — 2026-09-07 — canonical tracked real-function exponents

Canonical exact results from built-in `sin`, `cos`, `ln`, `exp`, and `arctan`
may now provide bounded real-power exponents over tracked integer snapshots.
Noncanonical inputs, user overrides, and oversized results retain their runtime
calls and `f64_pow`.

