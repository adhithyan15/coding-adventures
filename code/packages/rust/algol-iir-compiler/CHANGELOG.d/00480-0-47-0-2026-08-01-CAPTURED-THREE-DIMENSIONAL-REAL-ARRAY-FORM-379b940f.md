## 0.47.0 — 2026-08-01 — captured three-dimensional real-array formals

Regression coverage now proves that a nested procedure can write a three-dimensional
`real array` value formal. The captured descriptor retains its `array<f64>` handle,
three non-unit lower bounds, and both row-major strides, while the caller sums four
distinct dynamic cells through `entier` to confirm the intended values survive.

