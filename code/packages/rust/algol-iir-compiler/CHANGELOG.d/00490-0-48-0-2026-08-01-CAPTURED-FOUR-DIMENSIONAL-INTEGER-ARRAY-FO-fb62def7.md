## 0.48.0 — 2026-08-01 — captured four-dimensional integer-array formals

Regression coverage now proves that a nested procedure can write a four-dimensional
`integer array` value formal. The captured descriptor retains its `array<i64>` handle,
four non-unit lower bounds, and all three row-major strides, while the caller sums four
dynamic corner cells to confirm that the intended values survive.

