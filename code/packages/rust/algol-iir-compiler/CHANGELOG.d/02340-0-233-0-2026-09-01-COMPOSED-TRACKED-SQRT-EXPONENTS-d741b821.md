## 0.233.0 — 2026-09-01 — composed tracked `sqrt` exponents

Exact integral built-in `sqrt` results over tracked integer arithmetic may now
compose through checked `+`, `-`, and `*` exponent trees before bounded
real-power unrolling. Inexact roots, user overrides, division, invalid
arithmetic, and results outside the existing cap retain `f64_pow`.

