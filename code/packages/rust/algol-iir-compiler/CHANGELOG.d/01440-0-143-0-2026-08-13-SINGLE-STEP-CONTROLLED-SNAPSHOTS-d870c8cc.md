## 0.143.0 — 2026-08-13 — single-step controlled snapshots

Exactly-one-iteration `step`/`until` elements now retain a finite static value
assigned directly to their controlled scalar by the body. Dynamic assignments,
compound bodies, and potentially repeating loops remain conservative.

