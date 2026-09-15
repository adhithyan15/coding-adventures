## 0.240.0 — 2026-09-08 — path-independent standard-function results

Pure built-in standard functions may now collapse distinct conditional tracked
real operands to one exact bounded exponent. The selector still executes;
differing results, effectful selectors, user overrides, and bare conditional
real operands retain `f64_pow`.

