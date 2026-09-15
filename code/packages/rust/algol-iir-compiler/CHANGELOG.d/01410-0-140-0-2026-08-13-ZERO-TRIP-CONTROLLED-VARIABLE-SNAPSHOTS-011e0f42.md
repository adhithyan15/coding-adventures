## 0.140.0 — 2026-08-13 — zero-trip controlled-variable snapshots

A statically empty `step`/`until` element now records the initial value assigned
to its controlled scalar after restoring unrelated entry snapshots. This keeps
formatter-free output aligned with the runtime loop preheader assignment.

