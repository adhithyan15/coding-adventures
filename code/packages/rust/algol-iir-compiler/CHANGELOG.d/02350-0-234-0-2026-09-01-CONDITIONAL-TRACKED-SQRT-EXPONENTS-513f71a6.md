## 0.234.0 — 2026-09-01 — conditional tracked `sqrt` exponents

Pure conditional exponent expressions with path-independent exact integral
built-in `sqrt` branches may now retain bounded real-power multiplication while
their selector still executes. Differing or inexact branches, effectful
selectors, and user overrides retain `f64_pow`.

