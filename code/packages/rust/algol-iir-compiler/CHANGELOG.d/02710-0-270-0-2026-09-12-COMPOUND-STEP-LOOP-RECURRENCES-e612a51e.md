## 0.270.0 — 2026-09-12 — compound step-loop recurrences

Finite `step`/`until` recurrence analysis now uses the exact single-assignment
extractor, allowing a scalar update wrapped in a one-statement `begin`/`end`
body to retain its terminal static snapshot. Labels, conditionals,
declarations, and multiple statements continue to fail closed.

