## 0.46.0 — 2026-08-01 — captured three-dimensional boolean-array formals

Regression coverage now proves that a nested procedure can write a three-dimensional
`boolean array` value formal. The captured descriptor retains its `array<bool>` handle,
three non-unit lower bounds, and both row-major strides, while checkerboard reads confirm
the caller observes the intended boolean cells.

