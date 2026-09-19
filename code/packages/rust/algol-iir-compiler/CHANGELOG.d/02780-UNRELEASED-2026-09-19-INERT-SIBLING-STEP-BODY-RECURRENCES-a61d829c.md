## Unreleased — 2026-09-19 — Inert sibling step-body recurrences

Finite `step`/`until` analysis now retains one exact local scalar body
recurrence when its compound body also contains only bare local scalar
self-assignments. Changing siblings and all effectful or dynamic shapes remain
conservative.
