## 0.227.0 — 2026-08-27 — tracked arithmetic real-power exponents

Real-base powers now retain bounded multiplication lowering when their
nonnegative exponent is checked integer arithmetic over exact local snapshots.
Invalid arithmetic, calls, conditionals, globals, invalidated values, and
oversized exponents remain on the dynamic `f64_pow` path.

