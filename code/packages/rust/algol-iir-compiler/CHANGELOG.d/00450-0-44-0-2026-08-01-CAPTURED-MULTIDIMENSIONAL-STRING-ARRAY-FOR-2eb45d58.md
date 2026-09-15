## 0.44.0 — 2026-08-01 — captured multidimensional string-array formals

Regression coverage now proves that a nested procedure can write a two-dimensional
`string array` value formal. The captured descriptor retains its `array<str>`
handle, two non-unit lower bounds, and row-major stride, while lexical and equality
checks confirm the caller observes the correct dynamic string cells.

