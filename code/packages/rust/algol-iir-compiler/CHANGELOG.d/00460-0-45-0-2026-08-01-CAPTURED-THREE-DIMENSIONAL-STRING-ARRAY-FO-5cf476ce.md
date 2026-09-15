## 0.45.0 — 2026-08-01 — captured three-dimensional string-array formals

Regression coverage now proves that a nested procedure can write a three-dimensional
`string array` value formal. The captured descriptor retains its `array<str>` handle,
three non-unit lower bounds, and both row-major strides, while lexical and equality
checks confirm the caller observes the correct dynamic string cells.

