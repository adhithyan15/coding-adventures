## 0.142.0 — 2026-08-13 — zero-trip while-loop snapshots

Statically false `while` elements now preserve entry scalar snapshots and
record the initial value assigned to their controlled scalar. Unknown and
potentially repeating `while` elements remain conservative.

