## 0.146.0 — 2026-08-13 — finite loop-body snapshots

Finite static `step`/`until` loops now retain path-independent integer, real,
and boolean constants established by a body that does not reference the
controlled scalar. Control-dependent bodies and every existing dynamic or
tracking barrier remain conservative.

