## 0.220.0 — 2026-08-27 — tracked integer exponents in real snapshots

Static real-value metadata now accepts bounded exact local integer snapshots as
power exponents. Runtime variable exponents still lower through `f64_pow`;
invalidated, fractional, overflowing, and oversized values remain conservative.

