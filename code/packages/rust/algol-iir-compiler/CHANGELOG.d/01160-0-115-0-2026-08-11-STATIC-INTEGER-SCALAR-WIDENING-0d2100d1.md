## 0.115.0 — 2026-08-11 — static integer scalar widening

Straight-line local integer literals and copies now retain exact `i64`
snapshots for bounded static real expressions. Widening remains restricted to
binary64's exact integer range, and the metadata is cleared at the same
control-flow, call, capture, and dynamic-assignment boundaries as static reals.

