## Unreleased — 2026-09-19 — Inert sibling while recurrences

Bounded `while` analysis now retains one exact local scalar recurrence when its
compound body also contains only bare local scalar self-assignments. Changing
siblings and all effectful or dynamic shapes remain conservative.
