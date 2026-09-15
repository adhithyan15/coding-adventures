## 0.235.0 — 2026-09-01 — nested tracked `sqrt` exponents

Exact integral built-in `sqrt` results may now feed another tracked `sqrt`
before bounded real-power unrolling. Inexact or invalid intermediate roots and
user overrides retain `f64_pow`.

