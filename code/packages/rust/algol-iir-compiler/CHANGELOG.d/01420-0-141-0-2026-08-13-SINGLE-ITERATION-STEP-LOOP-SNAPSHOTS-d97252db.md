## 0.141.0 — 2026-08-13 — single-iteration step-loop snapshots

Statically proven single-iteration `step`/`until` elements now retain scalar
constants established by a body that does not reference the controlled
variable. Multi-iteration loops, calls, and other barriers remain conservative.

