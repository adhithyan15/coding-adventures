## Unreleased — 2026-09-19 — Inert sibling control recurrences

Bounded `step`/`until` analysis now retains an exact integer or real controlled
scalar recurrence when its compound body also contains only bare local scalar
self-assignments. Changing siblings and all effectful or dynamic shapes remain
conservative.
