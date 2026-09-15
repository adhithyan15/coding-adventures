## 0.241.0 — 2026-09-08 — composed path-independent standard-function results

Exact path-independent results from a pure built-in may now feed another pure
built-in in a bounded real exponent. Runtime selectors still execute, while
differing results, effectful selectors, and user overrides retain `f64_pow`.

