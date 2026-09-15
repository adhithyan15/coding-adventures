## 0.51.0 — 2026-08-01 — forwarding captured four-dimensional real arrays

Regression coverage now proves that a nested procedure can forward a captured
four-dimensional `real array` value formal to a sibling array formal. The forwarding
call reloads the `array<f64>` handle, four non-unit lower bounds, and all three row-major
strides before the callee writes caller-visible floating-point cells.

