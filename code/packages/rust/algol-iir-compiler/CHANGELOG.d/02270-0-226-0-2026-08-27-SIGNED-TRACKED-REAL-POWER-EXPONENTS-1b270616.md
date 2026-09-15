## 0.226.0 — 2026-08-27 — signed tracked real-power exponents

Real-base powers now retain bounded multiplication lowering when their
nonnegative exponent is an optionally signed bare local integer snapshot.
Signed tracked scalar metadata now applies the sign instead of accidentally
discarding it. Tracked arithmetic, calls, conditionals, globals, invalidated
values, and oversized exponents remain on the dynamic `f64_pow` path.

