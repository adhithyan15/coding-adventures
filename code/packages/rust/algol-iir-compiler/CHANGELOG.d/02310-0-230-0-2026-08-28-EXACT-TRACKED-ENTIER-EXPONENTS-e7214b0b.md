## 0.230.0 — 2026-08-28 — exact tracked `entier` exponents

Real-base power unrolling now accepts built-in `entier` over exact tracked
integer arithmetic when integer-to-real widening is lossless. User overrides,
values outside binary64's exact integer range, invalid arithmetic, and results
outside the existing expansion cap remain on the dynamic `f64_pow` path.

