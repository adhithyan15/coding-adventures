## 0.236.0 — 2026-09-01 — tracked `sqrt` standard-function exponents

Exact integral tracked `sqrt` results may now feed built-in `abs`, `sign`, and
`entier` before bounded real-power unrolling. Inexact or invalid roots and user
overrides retain their runtime calls and `f64_pow`.

