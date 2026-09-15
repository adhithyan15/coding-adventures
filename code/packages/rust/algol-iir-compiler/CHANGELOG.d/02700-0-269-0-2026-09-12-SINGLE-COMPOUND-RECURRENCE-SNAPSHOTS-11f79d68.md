## 0.269.0 — 2026-09-12 — single-compound recurrence snapshots

Bounded while-loop recurrence analysis now unwraps an exact one-statement
`begin`/`end` body, allowing a scalar assignment to retain its terminal static
snapshot. Labels, conditionals, declarations, and multiple statements continue
to fail closed.

