## 0.49.0 — 2026-08-01 — captured four-dimensional string-array formals

Regression coverage now proves that a nested procedure can write a four-dimensional
`string array` value formal. The captured descriptor retains its `array<str>` handle,
four non-unit lower bounds, and all three row-major strides, while lexical ordering and
equality checks confirm the intended dynamic string cells survive.

