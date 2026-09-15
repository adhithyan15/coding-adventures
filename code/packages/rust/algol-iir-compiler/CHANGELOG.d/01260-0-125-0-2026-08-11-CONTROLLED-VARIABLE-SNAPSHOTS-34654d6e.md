## 0.125.0 — 2026-08-11 — controlled-variable snapshots

Plain single-value `for` elements now update the local controlled variable's
integer or real snapshot alongside the emitted assignment. Body expressions
therefore observe each element's current static value instead of stale entry
metadata.

