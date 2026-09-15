## 0.273.0 — 2026-09-13 — integer control recurrence snapshots

Finite `step`/`until` analysis now follows one exact assignment to an integer
controlled scalar across multiple passes and retains the checked value that
exits the bound. Simulation is capped at 4,096 passes; cycles, overflow,
unknown expressions, and non-exact bodies continue to fail closed.

