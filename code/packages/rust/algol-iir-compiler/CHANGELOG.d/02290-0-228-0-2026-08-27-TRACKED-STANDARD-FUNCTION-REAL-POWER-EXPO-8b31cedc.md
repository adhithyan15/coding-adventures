## 0.228.0 — 2026-08-27 — tracked standard-function real-power exponents

Real-base powers now retain bounded multiplication lowering when `abs` or
`sign` consumes checked integer arithmetic over exact local snapshots and
produces a nonnegative exponent within the expansion cap. User overrides,
invalid arithmetic, conditionals, globals, and invalidated values remain on
the dynamic `f64_pow` path.

