## 0.239.0 — 2026-09-07 — conditional tracked real standard-function exponents

Pure real-valued selectors may now guard path-independent exact built-in
standard-function exponent branches over tracked local real snapshots. The
selector still executes, while differing branches, effectful selectors, bare
tracked real branches, and oversized results retain `f64_pow`.

