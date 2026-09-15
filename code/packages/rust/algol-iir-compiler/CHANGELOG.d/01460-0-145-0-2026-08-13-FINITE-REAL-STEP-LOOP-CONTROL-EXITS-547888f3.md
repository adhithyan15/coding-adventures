## 0.145.0 — 2026-08-13 — finite real step-loop control exits

Finite static real `step`/`until` elements whose bodies avoid the controlled
scalar now retain its first post-limit value by simulating the emitted binary64
additions. Simulation is capped at 4,096 iterations and rejects non-finite or
non-progressing additions; larger, dynamic, and target-referencing loops remain
conservative.

