## Unreleased — 2026-09-19 — Computed inert sibling recurrences

Bounded finite `step`/`until` and `while` recurrence analysis now reuses the
existing integer, real, and boolean identity proofs for inert compound-body
siblings. The one-changing-assignment rule, effect exclusions, and 4,096-pass
cap remain unchanged.
