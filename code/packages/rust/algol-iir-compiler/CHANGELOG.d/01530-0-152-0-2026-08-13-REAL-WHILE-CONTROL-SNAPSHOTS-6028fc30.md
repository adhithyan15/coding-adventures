## 0.152.0 — 2026-08-13 — real while-control snapshots

Capped `while` control analysis now has an explicit real-valued conformance
proof. Finite binary64 updates retain the first control value whose predicate
is false, while rounded-away progress remains conservative.

