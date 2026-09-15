## 0.123.0 — 2026-08-11 — single-value loop initialization

Plain single-value `for` list elements now retain definite scalar-string
initialization from their body because each executes exactly once. `while` and
`step`/`until` elements remain conservative, and mixed lists join each element
in sequence.

