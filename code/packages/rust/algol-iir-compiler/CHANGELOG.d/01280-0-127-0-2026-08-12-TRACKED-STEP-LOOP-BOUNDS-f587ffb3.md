## 0.127.0 — 2026-08-12 — tracked step-loop bounds

The statically nonempty `step`/`until` proof now consumes straight-line integer
or real snapshots from the loop header before dynamic-loop lowering invalidates
numeric metadata. Unknown and statically zero-trip bounds remain fail-closed,
and snapshots are still disabled before the loop body is analyzed.

