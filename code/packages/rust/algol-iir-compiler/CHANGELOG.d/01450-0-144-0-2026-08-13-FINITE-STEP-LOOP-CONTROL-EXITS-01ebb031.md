## 0.144.0 — 2026-08-13 — finite step-loop control exits

Finite static integer `step`/`until` elements whose bodies avoid the controlled
scalar now retain its first post-limit value using checked widened arithmetic.
Exactly-one elements with a direct static controlled assignment retain the
post-increment exit value rather than the pre-increment body value. Floating
multi-iteration progress, overflow, zero steps, arrays, globals, by-name targets,
dynamic writes, and target-referencing multi-iteration bodies remain conservative.

