## 0.274.0 — 2026-09-13 — real control recurrence snapshots

Finite `step`/`until` analysis now follows one exact assignment to a real
controlled scalar across multiple passes and retains the finite binary64 value
that exits the bound. Simulation remains capped at 4,096 passes; cycles,
rounded-away progress, unknown expressions, and non-finite results fail closed.

