## 0.224.0 — 2026-08-27 — tracked integer power exponents

Bounded integer power lowering now accepts exact tracked local scalar
exponents. User procedure calls, globals, invalidated values, overflow, and values
above the existing expansion cap still retain the dynamic real-power path.

