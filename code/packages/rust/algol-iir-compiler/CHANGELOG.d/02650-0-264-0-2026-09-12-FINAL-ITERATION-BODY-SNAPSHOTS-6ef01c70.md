## 0.264.0 — 2026-09-12 — final-iteration body snapshots

Statically bounded `step` loops now evaluate one simple scalar body assignment
at the final in-range control value. Integer and real controls, ascending and
descending steps, checked arithmetic, and the existing real-loop cap remain
conservative; writes to the control, compound bodies, and dynamic bounds fail
closed.

