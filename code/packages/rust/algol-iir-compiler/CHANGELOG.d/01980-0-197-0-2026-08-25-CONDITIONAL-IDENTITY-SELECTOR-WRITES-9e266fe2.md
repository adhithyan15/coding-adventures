## 0.197.0 — 2026-08-25 — conditional identity selector writes

The terminal bounded selector-dependency proof now recognizes conditional
assignments when every reachable leaf is a complete supported identity of the
same scalar. A changing leaf still fails closed and does not extend the
ten-level selector proof budget.

