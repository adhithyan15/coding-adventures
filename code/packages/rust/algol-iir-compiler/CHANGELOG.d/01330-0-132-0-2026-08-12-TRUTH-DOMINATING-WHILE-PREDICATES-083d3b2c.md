## 0.132.0 — 2026-08-12 — truth-dominating while predicates

The initial `while` proof now applies the sound truth-dominating identities
`false and x`, `true or x`, and `false impl x` without requiring `x` to be
statically known. Non-dominating forms and `eqv` remain conservative.

