## 0.272.0 — 2026-09-13 — dependent controlled-scalar exit snapshots

Single-iteration `step`/`until` analysis now seeds the controlled scalar's
known entry snapshot before evaluating its sole exact assignment. Checked
post-body increments that immediately exit retain their value; unknown
dependencies and assignments that can repeat continue to fail closed.

