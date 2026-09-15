## 0.139.0 — 2026-08-13 — zero-trip loop snapshot preservation

Statically proven empty `step`/`until` elements now preserve their entry
integer, real, and boolean snapshots. Unknown, `while`, and potentially
repeating elements remain conservative.

