## 0.225.0 — 2026-08-27 — bare tracked real-power exponents

Real-base powers now retain bounded multiplication lowering when their
nonnegative exponent is one exact bare local integer snapshot. Calls,
conditionals, globals, invalidated values, and oversized exponents remain on
the dynamic `f64_pow` path.

