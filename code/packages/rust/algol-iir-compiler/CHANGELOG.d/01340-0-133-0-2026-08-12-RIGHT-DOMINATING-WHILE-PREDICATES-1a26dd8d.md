## 0.133.0 — 2026-08-12 — right-dominating while predicates

The initial `while` proof now recognizes the symmetric truth-dominating forms
`x and false`, `x or true`, and `x impl true` when `x` is unknown. The
non-dominating right-literal forms remain conservative.

