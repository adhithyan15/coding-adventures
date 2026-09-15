## 0.334.0 — 2026-09-12 — ALGOL compound control-exit snapshots

The seven-backend ALGOL matrix now proves that a single-iteration
`step`/`until` loop retains its controlled scalar's checked exit snapshot when
the exact assignment is wrapped in a compound body.

